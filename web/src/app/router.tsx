import { createBrowserRouter } from "react-router-dom";

import { NotFoundPage } from "@/pages/NotFoundPage";
import { IpExtractRoute } from "@/routes/IpExtractRoute";
import { OverviewRoute } from "@/routes/OverviewRoute";
import { RootRoute } from "@/routes/RootRoute";
import { SessionsRoute } from "@/routes/SessionsRoute";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <RootRoute />,
    children: [
      { index: true, element: <OverviewRoute /> },
      { path: "ips", element: <IpExtractRoute /> },
      { path: "sessions", element: <SessionsRoute /> },
      { path: "*", element: <NotFoundPage /> },
    ],
  },
]);
