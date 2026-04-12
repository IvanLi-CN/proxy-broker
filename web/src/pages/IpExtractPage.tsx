import { RadarIcon, ScanSearchIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { IpFiltersForm } from "@/features/ips/components/IpFiltersForm";
import { IpResultsTable } from "@/features/ips/components/IpResultsTable";
import { useI18n } from "@/i18n";
import { formatGeoLabel, formatSortMode } from "@/lib/format";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";

interface IpExtractPageProps {
  isPending: boolean;
  response?: ExtractIpResponse | null;
  error?: string | null;
  lastRequest?: ExtractIpRequest | null;
  onSubmit: (payload: ExtractIpRequest) => void | Promise<void>;
}

function summarizeRequest(
  request: ExtractIpRequest | null | undefined,
  locale: ReturnType<typeof useI18n>["locale"],
  count: number,
  formatNumber: (value: number) => string,
  t: ReturnType<typeof useI18n>["t"],
) {
  if (!request) {
    return [
      count > 0
        ? t(count === 1 ? "{count} row" : "{count} rows", { count: formatNumber(count) })
        : t("No request yet"),
      t("Use the filter builder to create a candidate slice"),
    ];
  }

  const chips = [
    t(count === 1 ? "{count} row" : "{count} rows", { count: formatNumber(count) }),
    t("sort: {sortMode}", { sortMode: formatSortMode(request.sort_mode ?? "lru", t) }),
  ];

  if (request.country_codes?.length) {
    chips.push(t("countries: {countries}", { countries: request.country_codes.join(", ") }));
  }
  if (request.cities?.length) {
    chips.push(
      t("cities: {cities}", {
        cities: request.cities.map((city) => formatGeoLabel(locale, city) ?? city).join(", "),
      }),
    );
  }
  if (request.specified_ips?.length) {
    chips.push(t("include: {count}", { count: formatNumber(request.specified_ips.length) }));
  }
  if (request.blacklist_ips?.length) {
    chips.push(t("blacklist: {count}", { count: formatNumber(request.blacklist_ips.length) }));
  }
  if (request.limit) {
    chips.push(t("limit: {count}", { count: formatNumber(request.limit) }));
  }

  return chips;
}

export function IpExtractPage({
  isPending,
  response,
  error,
  lastRequest,
  onSubmit,
}: IpExtractPageProps) {
  const { formatNumber, locale, t } = useI18n();
  const resultCount = response?.items.length ?? 0;

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("IP Extract")}</h1>
      </header>

      <section className="grid gap-6 xl:grid-cols-[420px_minmax(0,1fr)]">
        <div>
          <IpFiltersForm isPending={isPending} onSubmit={onSubmit} />
        </div>

        <div className="space-y-4">
          {error ? (
            <ActionResponsePanel title={t("Extraction failed")} description={error} tone="error" />
          ) : null}
          <DataTablePanel
            eyebrow={t("Result deck")}
            title={t("Extracted candidates")}
            description={t(
              "Each surviving row reflects the current request plus the latest probe and location metadata returned by the backend.",
            )}
            chips={summarizeRequest(lastRequest, locale, resultCount, formatNumber, t)}
            actions={
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
              >
                <ScanSearchIcon className="mr-1 size-3.5" />
                {isPending ? t("running") : t("idle")}
              </Badge>
            }
          >
            <div className="space-y-4">
              <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                <RadarIcon className="size-4 text-primary" />
                {t("Candidate table")}
              </div>
              <IpResultsTable items={response?.items ?? []} isLoading={isPending} />
            </div>
          </DataTablePanel>
        </div>
      </section>
    </div>
  );
}
