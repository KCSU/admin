// A row of mutually exclusive options, like radio buttons.

export default function Segmented<T extends string>({ label, options, value, onChange }: {
  label: string; options: { id: T; label: string }[]; value: T; onChange: (value: T) => void;
}) {
  return (
    <div role="radiogroup" aria-label={label} class="inline-flex flex-wrap rounded-control border bg-muted/50 p-0.5">
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          role="radio"
          aria-checked={value === option.id}
          class={`rounded-control px-2.5 py-1 text-sm transition-colors ${
            value === option.id ? 'bg-background font-medium shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
          onClick={() => onChange(option.id)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
