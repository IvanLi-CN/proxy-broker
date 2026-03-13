import type { Preview } from "@storybook/react-vite";

import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "../src/index.css";

import { withAppProviders } from "@/stories/withAppProviders";

const preview: Preview = {
  tags: ["autodocs"],
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    layout: "padded",
    backgrounds: {
      default: "slate",
      values: [
        { name: "slate", value: "#e8edf4" },
        { name: "ink", value: "#101828" },
      ],
    },
  },
  globalTypes: {
    theme: {
      description: "Color theme",
      defaultValue: "light",
      toolbar: {
        title: "Theme",
        items: [
          { value: "light", title: "Light" },
          { value: "dark", title: "Dark" },
        ],
      },
    },
  },
  decorators: [withAppProviders],
};

export default preview;
