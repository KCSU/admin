// Stacked severity bars with alert markers, hover details and drag to zoom.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';

import { DAY, HOUR_MINUTE, LEVELS, type LogEntry, type Severity, type Volume } from '../logs';

const MINUTE = 60_000;
/** Below this width (px): a shorter chart, wider bars, no drag to zoom. */
const NARROW = 640;
/** Room above the plot for the top label and the alert diamonds. */
const TOP = 10;
/** Room right of the plot, so the last x label isn't cut off. */
const RIGHT = 32;
const Y_AXIS = 48;
const X_AXIS = 28;
const MIN_SEGMENT_PX = 2;
const MIN_DRAG_PX = 4;

export interface Alert {
  label: string;
  at: number;
}

interface Hover {
  /** From the plot's left edge. */
  left: number;
  slot: number;
}

export default function LogVolume({ entries, volume, alerts, levels, from, to, onZoom }: {
  entries: LogEntry[]; volume: Volume | null; alerts: Alert[]; levels: Set<Severity>; from: number; to: number;
  onZoom: (from: number, to: number) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [hover, setHover] = useState<Hover | null>(null);

  useEffect(() => {
    const observer = new ResizeObserver(([entry]) => setWidth(Math.floor(entry.contentRect.width)));
    observer.observe(container.current!);
    return () => observer.disconnect();
  }, []);

  const narrow = width < NARROW;
  const plotHeight = narrow ? 160 : 224;
  const plotWidth = width - Y_AXIS;
  const maxBuckets = Math.max(24, Math.min(72, Math.round(plotWidth / 20)));
  const { start, bucketMs, counts } = useMemo(() => {
    if (volume) {
      const merge = Math.ceil(volume.counts.length / maxBuckets);
      const merged = Array.from({ length: Math.ceil(volume.counts.length / merge) }, emptyCounts);
      volume.counts.forEach((slot, i) => {
        for (const level of LEVELS) merged[Math.floor(i / merge)][level.id] += slot[level.id];
      });
      return { start: volume.start, bucketMs: volume.periodMs * merge, counts: merged };
    }
    const bucketMs = (to - from) / maxBuckets;
    const counts = Array.from({ length: maxBuckets }, emptyCounts);
    for (const entry of entries) {
      const bucket = counts[Math.floor((entry.timestamp - from) / bucketMs)];
      if (bucket) bucket[entry.severity] += 1;
    }
    return { start: from, bucketMs, counts };
  }, [entries, volume, from, to, maxBuckets]);

  useEffect(() => {
    if (!width) return;
    const element = container.current!;
    const colors = readColors(element);
    const font = `14px ${getComputedStyle(element).fontFamily}`;

    const visible = LEVELS.filter((level) => levels.size === 0 || levels.has(level.id));
    const max = niceMax(Math.max(1, ...counts.map((c) => visible.reduce((sum, l) => sum + c[l.id], 0))));
    const minSegment = (max * MIN_SEGMENT_PX) / plotHeight;
    let running = counts.map(() => 0);
    const totals = visible.map((level) => {
      running = running.map((total, i) => total + (counts[i][level.id] ? Math.max(counts[i][level.id], minSegment) : 0));
      return running.map((total) => total || null);
    }).reverse();
    const xs = counts.map((_, i) => start + (i + 0.5) * bucketMs);
    const ticksAt = ticks(start, to, Math.max(3, Math.floor(plotWidth / 110)));
    const shown = alerts.filter((alert) => alert.at >= start && alert.at <= to);
    const dash = [3 * uPlot.pxRatio, 3 * uPlot.pxRatio];

    const plot = new uPlot({
      width: width + RIGHT,
      height: TOP + plotHeight + X_AXIS,
      padding: [TOP, RIGHT, 0, 0],
      legend: { show: false },
      cursor: {
        x: false,
        y: false,
        points: { show: false },
        drag: { x: !narrow, y: false, setScale: false },
      },
      scales: { x: { time: false, range: [start, to] }, y: { range: [0, max] } },
      axes: [
        {
          size: X_AXIS, gap: 8, font, stroke: colors.muted, grid: { show: false }, ticks: { show: false },
          splits: () => ticksAt,
          values: (_, splits) => splits.map((at, i) => (
            i === 0 ? `${DAY.format(at)} ${HOUR_MINUTE.format(at)} UTC`
              : DAY.format(at) !== DAY.format(splits[i - 1]) ? `${DAY.format(at)} ${HOUR_MINUTE.format(at)}`
              : HOUR_MINUTE.format(at)
          )),
        },
        {
          size: Y_AXIS, gap: 8, font, stroke: colors.muted, grid: { show: false }, ticks: { show: false },
          splits: () => [0, max / 2, max],
          values: (_, splits) => splits.map(String),
        },
      ],
      series: [
        {},
        ...[...visible].reverse().map((level) => ({
          fill: colors.levels[level.id],
          width: 0,
          alpha: 0.8,
          points: { show: false },
          paths: uPlot.paths.bars!({ size: [1, Infinity], gap: 1 }),
        })),
      ],
      hooks: {
        drawAxes: [(u) => {
          const { ctx } = u;
          const line = (y: number) => {
            ctx.beginPath();
            ctx.moveTo(u.bbox.left, Math.round(y) + 0.5);
            ctx.lineTo(u.bbox.left + u.bbox.width, Math.round(y) + 0.5);
            ctx.stroke();
          };
          ctx.save();
          ctx.lineWidth = uPlot.pxRatio;
          for (const [value, stroke, lineDash] of [[max, colors.grid, dash], [max / 2, colors.grid, dash], [0, colors.border, []]] as const) {
            ctx.strokeStyle = stroke;
            ctx.setLineDash(lineDash);
            line(u.valToPos(value, 'y', true));
          }
          ctx.restore();
        }],
        draw: [(u) => {
          const { ctx } = u;
          const top = u.bbox.top;
          const half = 4 * Math.SQRT2 * uPlot.pxRatio;
          ctx.save();
          ctx.strokeStyle = colors.alert;
          ctx.fillStyle = colors.alert;
          ctx.lineWidth = uPlot.pxRatio;
          for (const alert of shown) {
            const x = Math.round(u.valToPos(alert.at, 'x', true)) + 0.5;
            ctx.setLineDash(dash);
            ctx.beginPath();
            ctx.moveTo(x, top);
            ctx.lineTo(x, top + u.bbox.height);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(x, top - half);
            ctx.lineTo(x + half, top);
            ctx.lineTo(x, top + half);
            ctx.lineTo(x - half, top);
            ctx.closePath();
            ctx.fill();
          }
          ctx.restore();
        }],
        setCursor: [(u) => {
          const i = u.cursor.idx;
          const left = u.cursor.left;
          if (i == null || left == null || left < 0) {
            setHover(null);
            return;
          }
          setHover({ left, slot: i });
        }],
        setSelect: [(u) => {
          const { left, width: dragged } = u.select;
          if (dragged >= MIN_DRAG_PX) onZoom(u.posToVal(left, 'x'), u.posToVal(left + dragged, 'x'));
          u.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
        }],
      },
    }, [xs, ...totals], element);
    return () => plot.destroy();
  }, [start, bucketMs, counts, alerts, levels, to, width]);

  return (
    <div class="relative mt-0.5" aria-hidden="true">
      <div ref={container} class="log-volume -mr-8 pr-8" />
      {hover && counts[hover.slot] && (
        <Tooltip
          start={start + hover.slot * bucketMs}
          end={start + (hover.slot + 1) * bucketMs}
          counts={counts[hover.slot]}
          alerts={alerts.filter((alert) => alert.at <= to)}
          left={Y_AXIS + hover.left}
          flip={hover.left > plotWidth / 2}
        />
      )}
    </div>
  );
}

function Tooltip({ start, end, counts, alerts, left, flip }: {
  start: number; end: number; counts: Record<Severity, number>; alerts: Alert[]; left: number; flip: boolean;
}) {
  const present = LEVELS.filter((level) => counts[level.id] > 0);
  const here = alerts.filter((alert) => alert.at >= start && alert.at < end);
  return (
    <div
      class={`pointer-events-none absolute z-10 min-w-44 rounded-control border bg-popover px-3 py-2 text-xs shadow-lg shadow-black/40 ${flip ? '-translate-x-full' : ''}`}
      style={{ left: flip ? left - 12 : left + 12, top: TOP }}
    >
      <div class="mb-1.5 font-medium tabular-nums">
        {DAY.format(start)} {HOUR_MINUTE.format(start)}–{HOUR_MINUTE.format(end)} UTC
      </div>
      {present.length === 0 ? (
        <div class="text-muted-foreground">No lines</div>
      ) : present.map((level) => (
        <div key={level.id} class="flex items-center gap-2">
          <span class={`h-1 w-3.5 rounded-full ${level.color}`} />
          <span class="text-muted-foreground">{level.label}</span>
          <span class="ml-auto pl-4 tabular-nums">{counts[level.id].toLocaleString('en-GB')}</span>
        </div>
      ))}
      {here.map((alert) => (
        <div key={alert.at} class="mt-1.5 flex items-center gap-2 border-t pt-1.5">
          <span class="size-2 shrink-0 rotate-45 bg-red-400" />
          <span>{alert.label}</span>
        </div>
      ))}
    </div>
  );
}

function emptyCounts(): Record<Severity, number> {
  return { DEFAULT: 0, DEBUG: 0, INFO: 0, WARNING: 0, ERROR: 0 };
}

/** Canvas colours. */
function readColors(parent: HTMLElement) {
  const read = (className: string, property: 'color' | 'backgroundColor' | 'borderTopColor') => {
    const probe = document.createElement('span');
    probe.className = className;
    parent.append(probe);
    const value = getComputedStyle(probe)[property];
    probe.remove();
    return value;
  };
  return {
    muted: read('text-muted-foreground', 'color'),
    grid: read('border-border/60', 'borderTopColor'),
    border: read('border-border', 'borderTopColor'),
    alert: read('bg-red-400', 'backgroundColor'),
    levels: Object.fromEntries(LEVELS.map((level) => [level.id, read(level.color, 'backgroundColor')])) as Record<Severity, string>,
  };
}

const TICK_STEPS = [1, 2, 5, 10, 15, 30, 60, 2 * 60, 3 * 60, 6 * 60, 12 * 60, 24 * 60].map((minutes) => minutes * MINUTE);

/** Round UTC times for the x axis, at most `count` steps. */
function ticks(from: number, to: number, count: number): number[] {
  const step = TICK_STEPS.find((s) => (to - from) / s <= count) ?? TICK_STEPS[TICK_STEPS.length - 1];
  const out = [];
  for (let at = Math.ceil(from / step) * step; at < to; at += step) out.push(at);
  return out;
}

/** The next round number at or above `n`, for the y axis. */
function niceMax(n: number): number {
  const step = 10 ** Math.floor(Math.log10(n));
  return [1, 2, 4, 5, 10].map((m) => m * step).find((m) => m >= n)!;
}
