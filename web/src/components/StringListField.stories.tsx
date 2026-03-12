import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { StringListField } from "@/components/StringListField";

const meta = {
  title: "Components/StringListField",
  component: StringListField,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Textarea helper for comma or newline separated operator lists such as countries, cities, and IPs.",
      },
    },
  },
  render: (args) => {
    const [value, setValue] = useState(args.value);
    return <StringListField {...args} value={value} onChange={setValue} />;
  },
  args: {
    id: "country-codes",
    label: "Country codes",
    helper: "Comma or newline separated ISO country codes.",
    placeholder: "JP, US, SG",
    value: "JP, US",
    onChange: () => undefined,
  },
} satisfies Meta<typeof StringListField>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
