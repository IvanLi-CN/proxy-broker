import { CheckCircle2Icon, SirenIcon, TriangleAlertIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

interface ActionResponsePanelProps {
  title: string;
  tone?: "success" | "warning" | "error";
  description: string;
  bullets?: string[];
}

export function ActionResponsePanel({
  title,
  tone = "success",
  description,
  bullets,
}: ActionResponsePanelProps) {
  const Icon =
    tone === "error" ? SirenIcon : tone === "warning" ? TriangleAlertIcon : CheckCircle2Icon;

  return (
    <Alert variant={tone === "error" ? "destructive" : "default"}>
      <Icon className="size-4" />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        <p>{description}</p>
        {bullets && bullets.length > 0 ? (
          <ul className="mt-3 list-disc space-y-1 pl-5 text-xs md:text-sm">
            {bullets.map((bullet) => (
              <li key={bullet}>{bullet}</li>
            ))}
          </ul>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
