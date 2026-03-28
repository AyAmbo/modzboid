import { cn } from "../../../shared/lib/utils";
import { Switch } from "../../../shared/components/ui/switch";
import type { EnumOption } from "../types";

interface SettingFieldProps {
  label: string;
  value: string;
  description: string | null;
  settingType: string;  // 'bool' | 'int' | 'float' | 'string' | 'enum'
  min: number | null;
  max: number | null;
  defaultValue: string | null;
  enumOptions?: EnumOption[];
  isDirty?: boolean;
  onChange: (value: string) => void;
}

export function SettingField({
  label,
  value,
  description,
  settingType,
  min,
  max,
  defaultValue,
  enumOptions,
  isDirty,
  onChange,
}: SettingFieldProps) {
  return (
    <div className={cn(
      "flex items-start gap-4 py-2.5 px-3 rounded transition-colors",
      isDirty && "bg-primary/5"
    )}>
      {/* Label + description */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{label}</span>
          {isDirty && (
            <span className="text-xs text-primary">modified</span>
          )}
        </div>
        {description && (
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
            {description}
          </p>
        )}
        {defaultValue && (
          <span className="text-xs text-muted-foreground/60">
            Default: {defaultValue}
          </span>
        )}
      </div>

      {/* Input */}
      <div className="shrink-0 w-48">
        {settingType === "bool" ? (
          <Switch
            checked={value === "true"}
            onCheckedChange={(checked) => onChange(checked ? "true" : "false")}
          />
        ) : settingType === "enum" && enumOptions && enumOptions.length > 0 ? (
          <select
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="w-full px-2 py-1.5 text-sm bg-muted border border-border rounded"
          >
            {enumOptions.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        ) : settingType === "int" || settingType === "float" ? (
          <input
            type="number"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            min={min ?? undefined}
            max={max ?? undefined}
            step={settingType === "float" ? "0.01" : "1"}
            className="w-full px-2 py-1.5 text-sm bg-muted border border-border rounded font-mono"
          />
        ) : (
          <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="w-full px-2 py-1.5 text-sm bg-muted border border-border rounded"
          />
        )}
      </div>
    </div>
  );
}
