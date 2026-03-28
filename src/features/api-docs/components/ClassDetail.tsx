/** @deprecated This component is no longer used by ApiDocsPage. Docs are now rendered in-browser or via iframe. */
import { useState, useEffect } from "react";
import { cn } from "../../../shared/lib/utils";

export interface ApiField {
  name: string;
  fieldType: string;
  description: string;
}

export interface ApiParam {
  name: string;
  paramType: string;
  description: string;
}

export interface ApiMethod {
  name: string;
  params: ApiParam[];
  returnType: string;
  description: string;
  isStatic: boolean;
  isDeprecated: boolean;
}

export interface ApiClassDetail {
  name: string;
  kind: string; // "java" | "lua" | "event"
  parentClass: string | null;
  interfaces: string[];
  description: string;
  fields: ApiField[];
  methods: ApiMethod[];
}

interface Props {
  detail: ApiClassDetail;
}

const METHOD_LIMIT = 50; // Show first N methods, then "show all" button

export function ClassDetail({ detail }: Props) {
  // Auto-collapse fields/methods for large classes to prevent UI freeze
  const isLargeClass = detail.methods.length > 100 || detail.fields.length > 50;
  const [fieldsOpen, setFieldsOpen] = useState(!isLargeClass || detail.fields.length <= 50);
  const [methodsOpen, setMethodsOpen] = useState(!isLargeClass);
  // Reset state when switching to a different class
  useEffect(() => {
    const large = detail.methods.length > 100 || detail.fields.length > 50;
    setFieldsOpen(!large || detail.fields.length <= 50);
    setMethodsOpen(!large);
  }, [detail.name]);

  const kindColor =
    detail.kind === "java"
      ? "bg-blue-500/20 text-blue-400 border-blue-500/30"
      : detail.kind === "lua"
        ? "bg-yellow-500/20 text-yellow-400 border-yellow-500/30"
        : "bg-purple-500/20 text-purple-400 border-purple-500/30";

  // Group overloaded methods by name
  const methodGroups = detail.methods.reduce<Record<string, ApiMethod[]>>(
    (acc, method) => {
      if (!acc[method.name]) acc[method.name] = [];
      acc[method.name].push(method);
      return acc;
    },
    {}
  );

  return (
    <div className="flex flex-col gap-4 h-full overflow-auto p-4">
      {/* Header */}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-base font-semibold font-mono">{detail.name}</span>
          <span
            className={cn(
              "px-1.5 py-0.5 rounded text-xs border font-medium",
              kindColor
            )}
          >
            {detail.kind}
          </span>
          {detail.parentClass && (
            <span className="text-xs text-muted-foreground">
              extends{" "}
              <span className="font-mono text-foreground">{detail.parentClass}</span>
            </span>
          )}
        </div>
        {detail.interfaces.length > 0 && (
          <div className="text-xs text-muted-foreground">
            implements{" "}
            {detail.interfaces.map((iface, i) => (
              <span key={iface}>
                <span className="font-mono text-foreground">{iface}</span>
                {i < detail.interfaces.length - 1 && ", "}
              </span>
            ))}
          </div>
        )}
        {detail.description && (
          <p className="text-sm text-muted-foreground mt-1">{detail.description}</p>
        )}
      </div>

      {/* Fields */}
      {detail.fields.length > 0 && (
        <div className="border border-border rounded-lg bg-card">
          <button
            onClick={() => setFieldsOpen((o) => !o)}
            className="w-full flex items-center justify-between px-3 py-2 text-sm font-medium hover:bg-muted rounded-t-lg"
          >
            <span>Fields ({detail.fields.length})</span>
            <span className="text-muted-foreground text-xs">
              {fieldsOpen ? "▲" : "▼"}
            </span>
          </button>
          {fieldsOpen && (
            <FieldTable fields={detail.fields} />
          )}
        </div>
      )}

      {/* Methods */}
      {Object.keys(methodGroups).length > 0 && (
        <div className="border border-border rounded-lg bg-card">
          <button
            onClick={() => setMethodsOpen((o) => !o)}
            className="w-full flex items-center justify-between px-3 py-2 text-sm font-medium hover:bg-muted rounded-t-lg"
          >
            <span>Methods ({detail.methods.length})</span>
            <span className="text-muted-foreground text-xs">
              {methodsOpen ? "▲" : "▼"}
            </span>
          </button>
          {methodsOpen && (
            <MethodList methodGroups={methodGroups} />
          )}
        </div>
      )}

      {detail.fields.length === 0 && detail.methods.length === 0 && (
        <p className="text-sm text-muted-foreground">No members documented.</p>
      )}
    </div>
  );
}

const FIELD_LIMIT = 50;

function FieldTable({ fields }: { fields: ApiField[] }) {
  const [visibleCount, setVisibleCount] = useState(FIELD_LIMIT);
  const visible = fields.slice(0, visibleCount);
  const remaining = fields.length - visibleCount;

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-t border-border bg-muted/50">
            <th className="px-3 py-1.5 text-left font-medium text-xs">Name</th>
            <th className="px-3 py-1.5 text-left font-medium text-xs">Type</th>
            <th className="px-3 py-1.5 text-left font-medium text-xs">Description</th>
          </tr>
        </thead>
        <tbody>
          {visible.map((field) => (
            <tr key={field.name} className="border-t border-border hover:bg-muted/30">
              <td className="px-3 py-1.5 font-mono text-xs">{field.name}</td>
              <td className="px-3 py-1.5 font-mono text-xs text-blue-400">{field.fieldType}</td>
              <td className="px-3 py-1.5 text-xs text-muted-foreground">{field.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {remaining > 0 && (
        <button
          onClick={() => setVisibleCount((c) => c + FIELD_LIMIT)}
          className="w-full py-2 text-xs text-primary hover:bg-muted/50 transition-colors border-t border-border"
        >
          Load {Math.min(remaining, FIELD_LIMIT)} more fields ({remaining} remaining)
        </button>
      )}
    </div>
  );
}

function MethodList({ methodGroups }: { methodGroups: Record<string, ApiMethod[]> }) {
  const [visibleCount, setVisibleCount] = useState(METHOD_LIMIT);
  const entries = Object.entries(methodGroups);
  const visible = entries.slice(0, visibleCount);
  const remaining = entries.length - visibleCount;

  return (
    <div className="divide-y divide-border">
      {visible.map(([name, overloads]) => (
        <MethodGroup key={name} name={name} overloads={overloads} />
      ))}
      {remaining > 0 && (
        <button
          onClick={() => setVisibleCount((c) => c + METHOD_LIMIT)}
          className="w-full py-2 text-xs text-primary hover:bg-muted/50 transition-colors"
        >
          Load {Math.min(remaining, METHOD_LIMIT)} more ({remaining} remaining)
        </button>
      )}
    </div>
  );
}

function MethodGroup({ name, overloads }: { name: string; overloads: ApiMethod[] }) {
  // Start collapsed — user clicks to expand
  const [expanded, setExpanded] = useState(false);

  const firstOverload = overloads[0];

  return (
    <div className="px-3 py-2">
      <button
        onClick={() => setExpanded((o) => !o)}
        className="w-full text-left flex items-start gap-2"
      >
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            {firstOverload.isStatic && (
              <span className="text-xs text-muted-foreground">[static]</span>
            )}
            <span className="font-mono text-sm font-medium">{name}</span>
            {firstOverload.isDeprecated && (
              <span className="text-xs bg-orange-500/20 text-orange-400 border border-orange-500/30 px-1 rounded">
                deprecated
              </span>
            )}
            {overloads.length > 1 && (
              <span className="text-xs text-muted-foreground">
                {overloads.length} overloads
              </span>
            )}
          </div>
          {!expanded && (
            <div className="text-xs text-muted-foreground mt-0.5 truncate">
              <MethodSignature method={firstOverload} />
            </div>
          )}
        </div>
        <span className="text-muted-foreground text-xs shrink-0 mt-0.5">
          {expanded ? "▲" : "▼"}
        </span>
      </button>

      {expanded && (
        <div className="mt-2 space-y-3 pl-2 border-l border-border ml-1">
          {overloads.map((method, i) => (
            <div key={i} className="space-y-1">
              <div className="font-mono text-xs text-foreground">
                <MethodSignature method={method} />
              </div>
              {method.description && (
                <p className="text-xs text-muted-foreground">{method.description}</p>
              )}
              {method.params.length > 0 && (
                <div className="space-y-0.5">
                  {method.params.map((param) => (
                    <div key={param.name} className="flex gap-2 text-xs">
                      <span className="font-mono text-blue-400">{param.name}</span>
                      <span className="text-muted-foreground">{param.paramType}</span>
                      {param.description && (
                        <span className="text-muted-foreground">— {param.description}</span>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function MethodSignature({ method }: { method: ApiMethod }) {
  const params = method.params
    .map((p) => `${p.name}: ${p.paramType}`)
    .join(", ");
  return (
    <span>
      {method.name}({params}){" "}
      {method.returnType && (
        <span className="text-blue-400">→ {method.returnType}</span>
      )}
    </span>
  );
}
