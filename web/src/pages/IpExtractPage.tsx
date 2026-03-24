import { RadarIcon, ScanSearchIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { RouteHero } from "@/components/RouteHero";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { WorkflowRail } from "@/components/WorkflowRail";
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
        cities: request.cities
          .map((city) => formatGeoLabel(locale, city) ?? city)
          .join(", "),
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
      <RouteHero
        eyebrow={t("IP Extract")}
        title={t("IP Extract hero title")}
        description={t("IP Extract hero description")}
        badges={[
          {
            label: t("{count} candidate rows", { count: formatNumber(resultCount) }),
            tone: resultCount > 0 ? "positive" : "neutral",
          },
          {
            label: isPending ? t("extract running") : t("ready for request"),
            tone: isPending ? "warning" : "positive",
          },
          {
            label: error ? t("request error") : t("no active error"),
            tone: error ? "danger" : "neutral",
          },
        ]}
        aside={
          <WorkflowRail
            eyebrow={t("Filter loop")}
            title={t("Use a narrow feedback cycle")}
            steps={[
              {
                title: t("Start broad"),
                description: t("Set countries or cities before you start hand-picking IPs."),
              },
              {
                title: t("Read the metadata"),
                description: t(
                  "Probe and recency columns usually tell you whether to tighten or widen the filter.",
                ),
              },
              {
                title: t("Promote only the good rows"),
                description: t(
                  "Carry the shortlist into Sessions once the candidate deck looks credible.",
                ),
              },
            ]}
          />
        }
      />

      <section className="grid gap-6 xl:grid-cols-[420px_minmax(0,1fr)]">
        <div className="space-y-6">
          <IpFiltersForm isPending={isPending} onSubmit={onSubmit} />
          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                {t("Best practice")}
              </div>
              <CardTitle className="text-xl tracking-tight">
                {t("Filter-first, then judge the edges")}
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                {t(
                  "IP extraction works best when the request stays readable. Keep the request shape clear enough that you can explain why each row survived.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              <div className="rounded-2xl border border-border/70 bg-background/80 p-4 text-sm leading-6 text-muted-foreground">
                {t(
                  "Start broad with country codes, then tighten with cities or specified IPs once probe latency tells you where the fast edges are.",
                )}
              </div>
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
              >
                {t("mobile tables scroll horizontally")}
              </Badge>
            </CardContent>
          </Card>
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
