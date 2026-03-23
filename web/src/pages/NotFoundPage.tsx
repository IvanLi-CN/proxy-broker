import { GhostIcon } from "lucide-react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center gap-5 rounded-[2rem] border border-dashed border-border/80 bg-background/70 px-6 py-10 text-center">
      <div className="rounded-full border border-border/70 bg-muted p-4">
        <GhostIcon className="size-6 text-muted-foreground" />
      </div>
      <div className="space-y-2">
        <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary">404</div>
        <h1 className="text-3xl font-semibold tracking-tight">Nothing on this route yet</h1>
        <p className="max-w-xl text-sm text-muted-foreground md:text-base">
          The control surface only exposes Overview, Tasks, IP Extract, and Sessions right now.
        </p>
      </div>
      <Button asChild>
        <Link to="/">Back to overview</Link>
      </Button>
    </div>
  );
}
