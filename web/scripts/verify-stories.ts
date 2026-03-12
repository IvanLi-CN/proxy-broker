import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const ROOT = process.cwd();
const scopes = ["src/components/ui", "src/components", "src/features", "src/pages"];
const componentExtensions = new Set([".tsx"]);
const ignoredFiles = new Set(["index.ts", "index.tsx"]);
const problems: string[] = [];

async function walk(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        return walk(full);
      }
      return [full];
    }),
  );
  return files.flat();
}

function isComponentTarget(file: string) {
  const base = path.basename(file);
  const ext = path.extname(file);
  if (!componentExtensions.has(ext)) return false;
  if (ignoredFiles.has(base)) return false;
  if (base.endsWith(".stories.tsx") || base.endsWith(".test.tsx")) return false;
  if (
    file.includes("/hooks/") ||
    file.includes("/lib/") ||
    file.includes("/mocks/") ||
    file.includes("/providers/")
  )
    return false;
  if (file.includes("/features/") && !file.includes("/components/")) return false;
  return true;
}

for (const scope of scopes) {
  const scopePath = path.join(ROOT, scope);
  try {
    await stat(scopePath);
  } catch {
    continue;
  }

  const files = await walk(scopePath);
  for (const file of files.filter(isComponentTarget)) {
    const storyFile = file.replace(/\.tsx$/, ".stories.tsx");
    try {
      const content = await readFile(storyFile, "utf8");
      if (content.includes("!autodocs")) {
        problems.push(`${storyFile}: must not disable autodocs`);
      }
      if (!/tags:\s*\[(.|\n)*autodocs/.test(content)) {
        problems.push(`${storyFile}: missing tags: ["autodocs"]`);
      }
      if (!/title:\s*["'`]/.test(content)) {
        problems.push(`${storyFile}: missing explicit title`);
      }
      if (!/component:\s*[A-Za-z0-9_]/.test(content)) {
        problems.push(`${storyFile}: missing component reference`);
      }
      if (!/parameters:\s*\{[\s\S]*description:\s*\{[\s\S]*component:\s*["'`]/.test(content)) {
        problems.push(`${storyFile}: missing parameters.docs.description.component`);
      }
    } catch {
      problems.push(`${file}: missing colocated story file`);
    }
  }
}

if (problems.length > 0) {
  console.error("Story verification failed:\n");
  for (const problem of problems) {
    console.error(`- ${problem}`);
  }
  process.exit(1);
}

console.log("Story coverage looks good.");
