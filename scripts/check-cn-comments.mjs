import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const TARGET_DIRS = ["src", "src-tauri/src"];
const EXTENSIONS = new Set([".ts", ".vue", ".rs"]);
const IGNORE_FILES = new Set(["env.d.ts"]);

const CN_COMMENT_RE = /(\/\/[^\n]*[\u4e00-\u9fff])|(\/\*[\s\S]*?[\u4e00-\u9fff][\s\S]*?\*\/)/m;
const FUNCTION_RE = /(function\s+\w+)|(\bconst\s+\w+\s*=\s*(async\s*)?\([^)]*\)\s*=>)|(\basync\s+function\s+\w+)|(\bfn\s+\w+)/m;

function walk(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walk(fullPath));
      continue;
    }

    const ext = path.extname(entry.name);
    if (!EXTENSIONS.has(ext)) {
      continue;
    }

    if (IGNORE_FILES.has(entry.name)) {
      continue;
    }

    files.push(fullPath);
  }

  return files;
}

const missing = [];

for (const relDir of TARGET_DIRS) {
  const absDir = path.join(ROOT, relDir);
  if (!fs.existsSync(absDir)) {
    continue;
  }

  for (const file of walk(absDir)) {
    const content = fs.readFileSync(file, "utf8");
    // 仅检查包含函数实现的文件，避免把纯声明文件也当成强制目标。
    const hasFunction = FUNCTION_RE.test(content);
    if (hasFunction && !CN_COMMENT_RE.test(content)) {
      missing.push(path.relative(ROOT, file));
    }
  }
}

if (missing.length > 0) {
  console.error("以下文件包含方法实现但缺少中文注释，请补充后再提交：");
  for (const file of missing) {
    console.error(`- ${file}`);
  }
  process.exit(1);
}

console.log("中文注释检查通过。");
