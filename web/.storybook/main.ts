import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(ts|tsx|mdx)"],
  addons: ["@storybook/addon-links", "@storybook/addon-docs", "@storybook/addon-a11y"],
  framework: {
    name: "@storybook/react-vite",
    options: {
      strictMode: true,
    },
  },
  async viteFinal(config) {
    return {
      ...config,
      build: {
        ...config.build,
        chunkSizeWarningLimit: 1400,
      },
    };
  },
};

export default config;
