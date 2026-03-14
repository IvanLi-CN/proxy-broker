import type { Meta, StoryObj } from "@storybook/react-vite";
import { FileJsonIcon, Link2Icon } from "lucide-react";
import { useState } from "react";
import { expect, userEvent, within } from "storybook/test";

import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const meta = {
  title: "UI/Select",
  component: Select,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Dropdown primitive used for sort modes and source types.",
      },
    },
  },
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => {
    const [value, setValue] = useState("lru");
    return (
      <Select onValueChange={setValue} value={value}>
        <SelectTrigger aria-label="Sort mode" className="w-48">
          <SelectValue placeholder="Sort mode" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value="lru">LRU</SelectItem>
            <SelectItem value="mru">MRU</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    );
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const overlay = within(canvasElement.ownerDocument.body);
    await userEvent.click(canvas.getByRole("combobox", { name: /sort mode/i }));
    expect(await overlay.findByRole("option", { name: "LRU" })).toBeVisible();
    expect(await overlay.findByRole("option", { name: "MRU" })).toBeVisible();
  },
};

export const SourceType: Story = {
  render: () => {
    const [value, setValue] = useState("url");
    return (
      <Select onValueChange={setValue} value={value}>
        <SelectTrigger aria-label="Source type" size="lg" className="w-56">
          <SelectValue placeholder="Choose source type" />
        </SelectTrigger>
        <SelectContent size="lg">
          <SelectGroup>
            <SelectItem size="lg" value="url">
              <span className="flex items-center gap-2">
                <Link2Icon className="size-4" />
                URL
              </span>
            </SelectItem>
            <SelectItem size="lg" value="file">
              <span className="flex items-center gap-2">
                <FileJsonIcon className="size-4" />
                File path
              </span>
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    );
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const overlay = within(canvasElement.ownerDocument.body);
    await userEvent.click(canvas.getByRole("combobox", { name: /source type/i }));
    expect(await overlay.findByRole("option", { name: "URL" })).toBeVisible();
    expect(await overlay.findByRole("option", { name: "File path" })).toBeVisible();
  },
};

export const FieldSizes: Story = {
  render: () => {
    const [small, setSmall] = useState("lru");
    const [medium, setMedium] = useState("lru");
    const [large, setLarge] = useState("url");

    return (
      <div className="grid gap-4">
        <Select onValueChange={setSmall} value={small}>
          <SelectTrigger aria-label="Small field" size="sm" className="w-44">
            <SelectValue placeholder="Small" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="lru">LRU</SelectItem>
              <SelectItem value="mru">MRU</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>

        <Select onValueChange={setMedium} value={medium}>
          <SelectTrigger aria-label="Default field" className="w-48">
            <SelectValue placeholder="Default" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="lru">LRU</SelectItem>
              <SelectItem value="mru">MRU</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>

        <Select onValueChange={setLarge} value={large}>
          <SelectTrigger aria-label="Large field" size="lg" className="w-56">
            <SelectValue placeholder="Large" />
          </SelectTrigger>
          <SelectContent size="lg">
            <SelectGroup>
              <SelectItem size="lg" value="url">
                <span className="flex items-center gap-2">
                  <Link2Icon className="size-4" />
                  URL
                </span>
              </SelectItem>
              <SelectItem size="lg" value="file">
                <span className="flex items-center gap-2">
                  <FileJsonIcon className="size-4" />
                  File path
                </span>
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
    );
  },
};

export const LargeFieldOverlay: Story = {
  render: () => {
    const [value, setValue] = useState("url");

    return (
      <div className="max-w-xl rounded-[28px] border border-border/70 bg-background/80 p-6">
        <div className="space-y-2">
          <Label htmlFor="source-type-story">Source type</Label>
          <Select onValueChange={setValue} value={value}>
            <SelectTrigger
              id="source-type-story"
              aria-label="Source type field"
              size="lg"
              className="w-56 bg-card"
            >
              <SelectValue placeholder="Choose source type" />
            </SelectTrigger>
            <SelectContent size="lg">
              <SelectGroup>
                <SelectItem size="lg" value="url">
                  <span className="flex items-center gap-2">
                    <Link2Icon className="size-4" />
                    URL
                  </span>
                </SelectItem>
                <SelectItem size="lg" value="file">
                  <span className="flex items-center gap-2">
                    <FileJsonIcon className="size-4" />
                    File path
                  </span>
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
          <p className="min-h-12 text-xs leading-5 text-muted-foreground">
            URL mode fetches remotely; file mode resolves from the Rust host filesystem.
          </p>
        </div>
      </div>
    );
  },
};
