import { CheckIcon, ChevronsUpDownIcon, LoaderCircleIcon, XIcon } from "lucide-react";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Label } from "@/components/ui/label";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { useI18n } from "@/i18n";
import type { SessionOptionItem } from "@/lib/types";
import { cn } from "@/lib/utils";

interface SearchableMultiSelectProps {
  id: string;
  label: string;
  helper?: string;
  layout?: "stacked" | "inline";
  size?: "default" | "sm";
  placeholder: string;
  searchPlaceholder: string;
  emptyText: string;
  values: string[];
  disabled?: boolean;
  searchKey?: string;
  onChange: (values: string[]) => void;
  onSearch: (query: string) => Promise<SessionOptionItem[]>;
}

export function SearchableMultiSelect({
  id,
  label,
  helper,
  layout = "stacked",
  size = "default",
  placeholder,
  searchPlaceholder,
  emptyText,
  values,
  disabled = false,
  searchKey = "",
  onChange,
  onSearch,
}: SearchableMultiSelectProps) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [options, setOptions] = useState<SessionOptionItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [labelMap, setLabelMap] = useState<Record<string, SessionOptionItem>>({});
  const requestVersion = useRef(0);
  const onSearchRef = useRef(onSearch);
  const searchRequest = useMemo(
    () => ({
      key: searchKey,
      query: deferredQuery.trim(),
    }),
    [deferredQuery, searchKey],
  );

  useEffect(() => {
    onSearchRef.current = onSearch;
  }, [onSearch]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const currentVersion = ++requestVersion.current;
    setLoading(true);
    setError(null);

    void onSearchRef
      .current(searchRequest.query)
      .then((items) => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setOptions(items);
        setLabelMap((current) => {
          const next = { ...current };
          for (const item of items) {
            next[item.value] = item;
          }
          return next;
        });
      })
      .catch(() => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setOptions([]);
        setError(t("Could not load options"));
      })
      .finally(() => {
        if (requestVersion.current === currentVersion) {
          setLoading(false);
        }
      });
  }, [open, searchRequest, t]);

  const selectedItems = useMemo(
    () =>
      values.map((value) => ({
        value,
        label: labelMap[value]?.label ?? value,
        meta: labelMap[value]?.meta,
      })),
    [labelMap, values],
  );

  const triggerLabel = useMemo(() => {
    if (selectedItems.length === 0) {
      return placeholder;
    }
    if (selectedItems.length === 1) {
      return selectedItems[0]?.label ?? placeholder;
    }
    const [first, second] = selectedItems;
    if (selectedItems.length === 2) {
      return `${first?.label ?? ""}, ${second?.label ?? ""}`;
    }
    return `${first?.label ?? ""}, ${second?.label ?? ""} +${selectedItems.length - 2}`;
  }, [placeholder, selectedItems]);

  const toggleValue = (value: string) => {
    if (values.includes(value)) {
      onChange(values.filter((item) => item !== value));
      return;
    }
    onChange([...values, value]);
  };

  const removeValue = (value: string) => {
    onChange(values.filter((item) => item !== value));
  };

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setQuery("");
    }
  };

  const labelBlock = <Label htmlFor={id}>{label}</Label>;

  const controlBlock = (
    <div className="min-w-0 space-y-2">
      <Popover open={open} onOpenChange={handleOpenChange}>
        <PopoverTrigger asChild>
          <Button
            id={id}
            type="button"
            variant="outline"
            size={size === "sm" ? "sm" : "default"}
            role="combobox"
            aria-expanded={open}
            className={cn(
              "h-auto w-full justify-between text-left shadow-none",
              layout === "inline" ? "rounded-lg" : "rounded-xl px-3 py-3",
              selectedItems.length === 0 ? "text-muted-foreground" : "text-foreground",
            )}
            disabled={disabled}
          >
            <span className="truncate">{triggerLabel}</span>
            {loading ? (
              <LoaderCircleIcon className="size-4 shrink-0 animate-spin text-muted-foreground" />
            ) : (
              <ChevronsUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[var(--radix-popover-trigger-width)] min-w-72 overflow-hidden p-0">
          <Command shouldFilter={false}>
            <CommandInput placeholder={searchPlaceholder} value={query} onValueChange={setQuery} />
            <CommandList>
              {loading ? (
                <div className="flex items-center justify-center gap-2 px-3 py-6 text-sm text-muted-foreground">
                  <LoaderCircleIcon className="size-4 animate-spin" />
                  {t("Loading options…")}
                </div>
              ) : null}
              {!loading && error ? (
                <div className="px-3 py-4 text-sm text-destructive">{error}</div>
              ) : null}
              {!loading && !error && options.length === 0 ? (
                <CommandEmpty>{emptyText}</CommandEmpty>
              ) : null}
              {!loading && !error
                ? options.map((item) => {
                    const selected = values.includes(item.value);
                    return (
                      <CommandItem
                        key={item.value}
                        value={item.value}
                        onSelect={() => toggleValue(item.value)}
                      >
                        <CheckIcon
                          className={cn(
                            "size-4 text-primary transition-opacity",
                            selected ? "opacity-100" : "opacity-0",
                          )}
                        />
                        <div className="min-w-0 flex-1">
                          <div className="truncate text-sm">{item.label}</div>
                          {item.meta ? (
                            <div className="truncate text-xs text-muted-foreground">
                              {item.meta}
                            </div>
                          ) : null}
                        </div>
                      </CommandItem>
                    );
                  })
                : null}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      {selectedItems.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {selectedItems.map((item) => (
            <button
              key={item.value}
              type="button"
              className={cn(
                "inline-flex items-center gap-1 rounded-full border border-border bg-background text-foreground transition-colors hover:bg-muted",
                size === "sm" ? "px-2 py-0.5 text-[11px]" : "px-2.5 py-1 text-xs",
              )}
              onClick={() => removeValue(item.value)}
              title={item.meta ? `${item.label} · ${item.meta}` : item.label}
            >
              <span className="truncate">
                {item.meta ? `${item.label} · ${item.meta}` : item.label}
              </span>
              <XIcon className="size-3 text-muted-foreground" />
            </button>
          ))}
        </div>
      ) : null}
      {helper && layout === "inline" ? (
        <p className="text-xs leading-5 text-muted-foreground">{helper}</p>
      ) : null}
    </div>
  );

  return layout === "inline" ? (
    <div className="grid gap-2 md:grid-cols-[88px_minmax(0,1fr)] md:items-start md:gap-3">
      <div className={cn(size === "sm" ? "md:pt-1.5" : "md:pt-2")}>{labelBlock}</div>
      {controlBlock}
    </div>
  ) : (
    <div className="space-y-2">
      <div className="space-y-1">
        {labelBlock}
        {helper ? <p className="text-xs leading-5 text-muted-foreground">{helper}</p> : null}
      </div>
      {controlBlock}
    </div>
  );
}
