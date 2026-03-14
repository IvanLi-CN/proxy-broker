import { RadarIcon, ScanSearchIcon } from "lucide-react";
import { Link } from "react-router-dom";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { EmptyPanel } from "@/components/EmptyPanel";
import { RouteHero } from "@/components/RouteHero";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { WorkflowRail } from "@/components/WorkflowRail";
import { IpFiltersForm, type IpFiltersFormValues } from "@/features/ips/components/IpFiltersForm";
import { IpResultsTable } from "@/features/ips/components/IpResultsTable";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";

interface IpExtractPageProps {
  isPending: boolean;
  initialized: boolean;
  initializationLoading: boolean;
  profileId: string;
  filtersFormValues: IpFiltersFormValues;
  onFormValuesChange: (values: IpFiltersFormValues) => void;
  response?: ExtractIpResponse | null;
  error?: string | null;
  lastRequest?: ExtractIpRequest | null;
  onSubmit: (payload: ExtractIpRequest) => void | Promise<void>;
}

function summarizeRequest(request?: ExtractIpRequest | null, count = 0) {
  if (!request) {
    return [
      count > 0 ? `${count} rows` : "No request yet",
      "Use the filter builder to create a candidate slice",
    ];
  }

  const chips = [
    `${count} row${count === 1 ? "" : "s"}`,
    `sort: ${(request.sort_mode ?? "lru").toUpperCase()}`,
  ];

  if (request.country_codes?.length) {
    chips.push(`countries: ${request.country_codes.join(", ")}`);
  }
  if (request.cities?.length) {
    chips.push(`cities: ${request.cities.join(", ")}`);
  }
  if (request.specified_ips?.length) {
    chips.push(`include: ${request.specified_ips.length}`);
  }
  if (request.blacklist_ips?.length) {
    chips.push(`blacklist: ${request.blacklist_ips.length}`);
  }
  if (request.limit) {
    chips.push(`limit: ${request.limit}`);
  }

  return chips;
}

export function IpExtractPage({
  isPending,
  initialized,
  initializationLoading,
  profileId,
  filtersFormValues,
  onFormValuesChange,
  response,
  error,
  lastRequest,
  onSubmit,
}: IpExtractPageProps) {
  const resultCount = response?.items.length ?? 0;

  return (
    <div className="space-y-8">
      <RouteHero
        eyebrow="IP Extract"
        title="Slice the pool into a shortlist you can actually trust."
        description="Use the filter builder to move from broad geographic hints to a candidate deck with clear probe, latency, and recency signals. The goal is not more rows; it is better rows."
        badges={[
          {
            label: `${resultCount} candidate rows`,
            tone: resultCount > 0 ? "positive" : "neutral",
          },
          {
            label: initializationLoading
              ? `loading ${profileId}`
              : initialized
                ? "workspace ready"
                : "needs subscription",
            tone: initializationLoading ? "warning" : initialized ? "positive" : "warning",
          },
          {
            label: error ? "request error" : "no active error",
            tone: error ? "danger" : "neutral",
          },
        ]}
        aside={
          <WorkflowRail
            eyebrow="Filter loop"
            title="Use a narrow feedback cycle"
            steps={[
              {
                title: "Start broad",
                description: "Set countries or cities before you start hand-picking IPs.",
              },
              {
                title: "Read the metadata",
                description:
                  "Probe and recency columns usually tell you whether to tighten or widen the filter.",
              },
              {
                title: "Promote only the good rows",
                description:
                  "Carry the shortlist into Sessions once the candidate deck looks credible.",
              },
            ]}
          />
        }
      />

      {!initializationLoading && !initialized ? (
        <EmptyPanel
          title="Project not initialized"
          description="Load a subscription on the Overview page before you try to extract a shortlist for this project."
          icon={RadarIcon}
          hint="This page will restore your saved filter draft, but it needs a backend pool before any request can return rows."
        />
      ) : null}

      <section className="grid gap-6 xl:grid-cols-[420px_minmax(0,1fr)]">
        <div className="space-y-6">
          <IpFiltersForm
            initialValues={filtersFormValues}
            isPending={isPending || !initialized}
            onSubmit={onSubmit}
            onValuesChange={onFormValuesChange}
          />
          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                Best practice
              </div>
              <CardTitle className="text-xl tracking-tight">
                Filter-first, then judge the edges
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                IP extraction works best when the request stays readable. Keep the request shape
                clear enough that you can explain why each row survived.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              <div className="rounded-2xl border border-border/70 bg-background/80 p-4 text-sm leading-6 text-muted-foreground">
                Start broad with country codes, then tighten with cities or specified IPs once probe
                latency tells you where the fast edges are.
              </div>
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
              >
                mobile tables scroll horizontally
              </Badge>
              {!initialized ? (
                <Button asChild variant="outline" className="w-full justify-center">
                  <Link to="/">Go load a subscription</Link>
                </Button>
              ) : null}
            </CardContent>
          </Card>
        </div>

        <div className="space-y-4">
          {error ? (
            <ActionResponsePanel title="Extraction failed" description={error} tone="error" />
          ) : null}
          <DataTablePanel
            eyebrow="Result deck"
            title="Extracted candidates"
            description="Each surviving row reflects the current request plus the latest probe and location metadata returned by the backend."
            chips={summarizeRequest(lastRequest, resultCount)}
            actions={
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
              >
                <ScanSearchIcon className="mr-1 size-3.5" />
                {isPending ? "running" : "idle"}
              </Badge>
            }
          >
            <div className="space-y-4">
              <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                <RadarIcon className="size-4 text-primary" />
                Candidate table
              </div>
              <IpResultsTable
                items={response?.items ?? []}
                isLoading={isPending || initializationLoading}
              />
            </div>
          </DataTablePanel>
        </div>
      </section>
    </div>
  );
}
