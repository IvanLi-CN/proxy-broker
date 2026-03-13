import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

interface StringListFieldProps {
  id: string;
  label: string;
  helper: string;
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}

export function StringListField({
  id,
  label,
  helper,
  placeholder,
  value,
  onChange,
}: StringListFieldProps) {
  return (
    <div className="space-y-2">
      <div className="space-y-1">
        <Label htmlFor={id}>{label}</Label>
        <p className="text-xs text-muted-foreground">{helper}</p>
      </div>
      <Textarea
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="min-h-24 font-mono text-xs"
      />
    </div>
  );
}
