import {
  AlertTriangleIcon,
  KeyRoundIcon,
  LoaderCircleIcon,
  ShieldCheckIcon,
  ShieldEllipsisIcon,
  UserRoundXIcon,
  WrenchIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { useI18n } from "@/i18n";
import type { CurrentUserState } from "@/lib/types";
import { cn } from "@/lib/utils";

interface CurrentUserSummaryProps {
  currentUser: CurrentUserState;
  variant?: "compact" | "detail";
  className?: string;
}

export function CurrentUserSummary({
  currentUser,
  variant = "detail",
  className,
}: CurrentUserSummaryProps) {
  const { t } = useI18n();
  const summary = describeCurrentUser(currentUser, t);

  if (variant === "compact") {
    return (
      <div className={cn("flex flex-wrap items-center gap-2", className)}>
        <Badge
          variant="outline"
          className={cn(
            "rounded-full px-3 py-1 text-[11px] uppercase tracking-[0.18em]",
            summary.badgeClassName,
          )}
        >
          <summary.icon className={cn("mr-1.5 size-3.5", summary.iconClassName)} />
          {summary.shortLabel}
        </Badge>
        <Badge
          variant="outline"
          className="rounded-full bg-background/80 px-3 py-1 font-mono text-[11px]"
        >
          {summary.subjectLabel}
        </Badge>
        {summary.extraCompactBadges.map((badge) => (
          <Badge key={badge.label} className={cn("rounded-full", badge.className)}>
            {badge.label}
          </Badge>
        ))}
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border border-border/70 bg-muted/15 p-4", className)}>
      <div className="flex items-center gap-2 text-sm font-medium text-foreground">
        <summary.icon className={cn("size-4", summary.iconClassName)} />
        {t("Current user")}
      </div>
      <div className="mt-3">
        <div className="text-base font-semibold tracking-tight text-foreground">
          {summary.primaryLabel}
        </div>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{summary.description}</p>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        {summary.badges.map((badge) => (
          <Badge key={badge.label} className={cn("rounded-full", badge.className)}>
            {badge.label}
          </Badge>
        ))}
      </div>
      {summary.metaLines.length ? (
        <div className="mt-3 grid gap-1 text-sm text-muted-foreground">
          {summary.metaLines.map((line) => (
            <p key={line}>{line}</p>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function describeCurrentUser(currentUser: CurrentUserState, t: ReturnType<typeof useI18n>["t"]) {
  if (currentUser.status === "loading") {
    return {
      icon: LoaderCircleIcon,
      iconClassName: "animate-spin text-sky-500",
      shortLabel: t("loading"),
      subjectLabel: t("resolving identity"),
      primaryLabel: t("Resolving current user"),
      description: t("Waiting for /api/v1/auth/me so the UI can determine who is operating it."),
      badgeClassName: "border-sky-500/20 bg-sky-500/[0.09] text-sky-700 dark:text-sky-300",
      badges: [
        {
          label: t("loading"),
          className: "bg-sky-500/15 text-sky-700 dark:text-sky-300",
        },
      ],
      extraCompactBadges: [],
      metaLines: [],
    };
  }

  if (currentUser.status === "anonymous") {
    return {
      icon: UserRoundXIcon,
      iconClassName: "text-amber-500",
      shortLabel: t("anonymous"),
      subjectLabel: t("no forwarded identity"),
      primaryLabel: t("Anonymous browser session"),
      description: t(
        "No Forward Auth identity is available. The backend treats this browser as anonymous until a user session or forwarded headers appear.",
      ),
      badgeClassName: "border-amber-500/20 bg-amber-500/[0.09] text-amber-700 dark:text-amber-300",
      badges: [
        {
          label: t("anonymous"),
          className: "bg-amber-500/15 text-amber-700 dark:text-amber-300",
        },
        {
          label: t("protected actions blocked"),
          className: "bg-background/80 text-muted-foreground",
        },
      ],
      extraCompactBadges: [],
      metaLines: [t("Expected backend result: 401 authentication_required for protected routes.")],
    };
  }

  if (currentUser.status === "error") {
    return {
      icon: AlertTriangleIcon,
      iconClassName: "text-rose-500",
      shortLabel: t("identity error"),
      subjectLabel: t("status unavailable"),
      primaryLabel: t("Current user unavailable"),
      description: t(
        "The UI could not determine the current user from /api/v1/auth/me. This is different from an anonymous browser state.",
      ),
      badgeClassName: "border-rose-500/20 bg-rose-500/[0.09] text-rose-700 dark:text-rose-300",
      badges: [
        {
          label: t("error"),
          className: "bg-rose-500/15 text-rose-700 dark:text-rose-300",
        },
      ],
      extraCompactBadges: [],
      metaLines: [currentUser.message],
    };
  }

  const { identity } = currentUser;
  const principalTypeLabel =
    identity.principal_type === "api_key"
      ? t("api key")
      : identity.principal_type === "development"
        ? t("development")
        : t("human");
  const commonBadges = [
    {
      label: principalTypeLabel,
      className: "bg-background/80 font-mono text-[11px] text-foreground",
    },
  ];
  const commonMeta = [];

  if (identity.email) {
    commonMeta.push(t("Email: {email}", { email: identity.email }));
  }
  if (identity.groups.length) {
    commonMeta.push(t("Groups: {groups}", { groups: identity.groups.join(" / ") }));
  }

  switch (identity.principal_type) {
    case "development":
      return {
        icon: WrenchIcon,
        iconClassName: "text-sky-500",
        shortLabel: t("development"),
        subjectLabel: identity.subject,
        primaryLabel: identity.subject,
        description: t(
          "Development mode injected this local admin identity. It bypasses forwarded headers on purpose.",
        ),
        badgeClassName: "border-sky-500/20 bg-sky-500/[0.09] text-sky-700 dark:text-sky-300",
        badges: [
          ...commonBadges,
          {
            label: t("admin"),
            className: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
          },
        ],
        extraCompactBadges: [
          {
            label: t("admin"),
            className: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
          },
        ],
        metaLines: commonMeta,
      };
    case "api_key":
      return {
        icon: KeyRoundIcon,
        iconClassName: "text-violet-500",
        shortLabel: t("api key"),
        subjectLabel: identity.subject,
        primaryLabel: identity.subject,
        description: t("Machine principal resolved from a profile-scoped API key."),
        badgeClassName:
          "border-violet-500/20 bg-violet-500/[0.09] text-violet-700 dark:text-violet-300",
        badges: [
          ...commonBadges,
          ...(identity.profile_id
            ? [
                {
                  label: t("profile {profileId}", { profileId: identity.profile_id }),
                  className: "bg-background/80 text-muted-foreground",
                },
              ]
            : []),
        ],
        extraCompactBadges: identity.profile_id
          ? [
              {
                label: t("profile {profileId}", { profileId: identity.profile_id }),
                className: "bg-background/80 text-muted-foreground",
              },
            ]
          : [],
        metaLines: [
          ...(identity.api_key_id ? [t("API key ID: {id}", { id: identity.api_key_id })] : []),
          ...(identity.profile_id
            ? [t("Bound profile: {profileId}", { profileId: identity.profile_id })]
            : []),
        ],
      };
    default:
      return {
        icon: identity.is_admin ? ShieldCheckIcon : ShieldEllipsisIcon,
        iconClassName: identity.is_admin ? "text-emerald-500" : "text-amber-500",
        shortLabel: identity.is_admin ? t("human admin") : t("human"),
        subjectLabel: identity.subject,
        primaryLabel: identity.subject,
        description: identity.is_admin
          ? t("Forward Auth identified an administrator. The backend authorizes admin-only routes.")
          : t(
              "Forward Auth identified a human user, but the backend did not classify them as admin.",
            ),
        badgeClassName: identity.is_admin
          ? "border-emerald-500/20 bg-emerald-500/[0.09] text-emerald-700 dark:text-emerald-300"
          : "border-amber-500/20 bg-amber-500/[0.09] text-amber-700 dark:text-amber-300",
        badges: [
          ...commonBadges,
          {
            label: identity.is_admin ? t("admin") : t("non-admin"),
            className: identity.is_admin
              ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
              : "bg-amber-500/15 text-amber-700 dark:text-amber-300",
          },
        ],
        extraCompactBadges: identity.is_admin
          ? [
              {
                label: t("admin"),
                className: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
              },
            ]
          : [],
        metaLines: commonMeta,
      };
  }
}
