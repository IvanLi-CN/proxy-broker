import { RadarIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { IpFiltersForm } from "@/features/ips/components/IpFiltersForm";
import { IpResultsTable } from "@/features/ips/components/IpResultsTable";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";

interface IpExtractPageProps {
  isPending: boolean;
  response?: ExtractIpResponse | null;
  error?: string | null;
  onSubmit: (payload: ExtractIpRequest) => void | Promise<void>;
}

export function IpExtractPage({ isPending, response, error, onSubmit }: IpExtractPageProps) {
  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="text-xs font-semibold uppercase tracking-[0.28em] text-primary">
          IP pool extractor
        </div>
        <h1 className="text-3xl font-semibold tracking-tight text-foreground md:text-4xl">
          Slice the subscription down to operator-friendly IP candidates.
        </h1>
        <p className="max-w-3xl text-sm leading-7 text-muted-foreground md:text-base">
          Use geo filters, allow-lists, and blacklist fences to produce a shortlist. The backend
          returns probe and last-used metadata so you can decide whether to open directly or adjust
          the selector first.
        </p>
      </section>

      <ActionResponsePanel
        title="Best practice"
        description="Start broad with country codes, then tighten with cities or specified IPs once probe latency tells you where the fast edges are."
      />

      <section className="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <IpFiltersForm isPending={isPending} onSubmit={onSubmit} />
        <div className="space-y-4">
          {error ? (
            <ActionResponsePanel title="Extraction failed" description={error} tone="error" />
          ) : null}
          <div className="rounded-2xl border border-border/70 bg-background/70 p-4">
            <div className="mb-4 flex items-center gap-2 text-sm font-semibold text-foreground">
              <RadarIcon className="size-4 text-primary" />
              Extracted candidates
            </div>
            <IpResultsTable items={response?.items ?? []} />
          </div>
        </div>
      </section>
    </div>
  );
}
