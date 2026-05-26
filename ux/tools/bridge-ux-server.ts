// ux/tools/bridge-ux-server.ts
// SecureLLM Bridge — UX Design Mode MCP Tools
//
// Exposes UX specification tools to AI agents so they can generate
// UI components that follow the Bridge design system precisely.
//
// Integration: Add to securellm-mcp server's tool registry.

import { readFile, readdir } from "node:fs/promises";
import { join, basename, extname } from "node:path";
import { parse as parseYaml } from "yaml";

// ── Types ───────────────────────────────────────────────────────────

interface UxSpec {
  name: string;
  version: string;
  philosophy: {
    tone: string;
    purpose: string;
    memorable_element: string;
    target_audience: string[];
  };
  typography: Record<string, unknown>;
  color_system: Record<string, unknown>;
  layout_rules: Record<string, unknown>;
  animation: Record<string, unknown>;
  constraints: Record<string, unknown>;
  anti_patterns: string[];
  components?: Record<string, unknown>;
  validation_checklist?: string[];
}

interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
}

// ── Tool Definitions ───────────────────────────────────────────────

const UX_SPECS_DIR =
  process.env.BRIDGE_UX_SPECS_DIR || join(process.cwd(), "ux/specs");

export function getUxTools(): ToolDefinition[] {
  return [
    {
      name: "ux_list_specs",
      description:
        "List all available UX design specifications for SecureLLM Bridge components",
      inputSchema: {
        type: "object",
        properties: {},
      },
    },
    {
      name: "ux_get_spec",
      description:
        "Retrieve a complete UX specification for generating UI components with precise design rules",
      inputSchema: {
        type: "object",
        properties: {
          spec_name: {
            type: "string",
            description: "Name of the UX spec (e.g., 'bridge-gateway')",
          },
          format: {
            type: "string",
            enum: ["yaml", "json", "markdown", "prompt"],
            description: "Output format — 'prompt' generates an optimized agent prompt",
            default: "markdown",
          },
        },
        required: ["spec_name"],
      },
    },
    {
      name: "ux_generate_prompt",
      description:
        "Generate an optimized AI agent prompt for creating a specific UI component following Bridge design specs",
      inputSchema: {
        type: "object",
        properties: {
          spec_name: {
            type: "string",
            description: "UX spec name",
          },
          component: {
            type: "string",
            description: "Component to generate (e.g., 'provider_card', 'topology_graph')",
          },
          requirements: {
            type: "string",
            description: "Additional specific requirements for this component instance",
          },
        },
        required: ["spec_name", "component"],
      },
    },
    {
      name: "ux_validate_component",
      description:
        "Validate a generated component against Bridge UX specifications (colors, fonts, layout)",
      inputSchema: {
        type: "object",
        properties: {
          spec_name: {
            type: "string",
            description: "UX spec to validate against",
          },
          component_type: {
            type: "string",
            description: "Component type being validated",
          },
          code: {
            type: "string",
            description: "The generated component source code to validate",
          },
        },
        required: ["spec_name", "component_type", "code"],
      },
    },
    {
      name: "ux_design_system",
      description:
        "Get the complete Bridge Design System reference — colors, typography, spacing, and component patterns",
      inputSchema: {
        type: "object",
        properties: {
          section: {
            type: "string",
            enum: ["colors", "typography", "layout", "animation", "components", "all"],
            description: "Which section of the design system to retrieve",
            default: "all",
          },
        },
      },
    },
    {
      name: "ux_create_spec",
      description:
        "Create a new UX specification from a template for a Bridge component or view",
      inputSchema: {
        type: "object",
        properties: {
          name: {
            type: "string",
            description: "Name for the new UX spec",
          },
          purpose: {
            type: "string",
            description: "What this component/view does",
          },
          tone: {
            type: "string",
            description: "Aesthetic direction (e.g., 'terminal-native', 'industrial precision')",
          },
        },
        required: ["name", "purpose"],
      },
    },
  ];
}

// ── Tool Handlers ───────────────────────────────────────────────────

async function loadSpec(name: string): Promise<UxSpec> {
  const specPath = join(UX_SPECS_DIR, `${name}.yml`);
  const content = await readFile(specPath, "utf-8");
  return parseYaml(content) as UxSpec;
}

async function listSpecFiles(): Promise<string[]> {
  const files = await readdir(UX_SPECS_DIR);
  return files
    .filter((f) => f.endsWith(".yml") || f.endsWith(".yaml"))
    .map((f) => basename(f, extname(f)));
}

function specToMarkdown(spec: UxSpec): string {
  const lines: string[] = [
    `# ${spec.name} — UX Specification`,
    `**Version**: ${spec.version}`,
    "",
    "## 🎯 Design Philosophy",
    `- **Tone**: ${spec.philosophy.tone}`,
    `- **Purpose**: ${spec.philosophy.purpose}`,
    `- **Memorable Element**: ${spec.philosophy.memorable_element}`,
    `- **Audience**: ${spec.philosophy.target_audience.join(", ")}`,
    "",
    "## 🔤 Typography",
    `| Role | Font |`,
    `|------|------|`,
    `| Display | ${spec.typography.display_font} |`,
    `| Monospace | ${spec.typography.mono_font} |`,
    `| Body | ${spec.typography.body_font} |`,
    ...((spec.typography.rules as string[]) || []).map((r) => `- ${r}`),
    "",
    "## 🎨 Color System",
    "### Base",
    ...Object.entries(spec.color_system.base as Record<string, string>).map(
      ([k, v]) => `- \`${k}\`: \`${v}\``
    ),
    "### Accents",
    ...Object.entries(spec.color_system.accents as Record<string, string>).map(
      ([k, v]) => `- \`${k}\`: \`${v}\``
    ),
    "### Semantic",
    ...Object.entries(spec.color_system.semantic as Record<string, string>).map(
      ([k, v]) => `- \`${k}\`: \`${v}\``
    ),
    "",
    "## 📐 Layout Rules",
    `- Grid: ${(spec.layout_rules.grid as any).columns} columns, ${(spec.layout_rules.grid as any).gutter} gutters`,
    `- Direction: ${spec.layout_rules.direction}`,
    `- Section margin: ${(spec.layout_rules.spacing as any).section_margin}`,
    `- Card padding: ${(spec.layout_rules.spacing as any).card_padding}`,
    "",
    "## 🎬 Animation",
    `- Page load: ${(spec.animation as any).page_load.strategy} (${(spec.animation as any).page_load.stagger}s stagger)`,
    `- Hover: ${(spec.animation as any).interactions.hover_effect} (${(spec.animation as any).interactions.hover_duration}ms)`,
    "",
    "## 🚫 Anti-Patterns",
    ...spec.anti_patterns.map((a) => `- ❌ ${a}`),
    "",
  ];

  if (spec.components) {
    lines.push("## 🧩 Components");
    for (const [name, comp] of Object.entries(spec.components)) {
      lines.push(
        `### ${name}`,
        `${(comp as any).description || ""}`,
        "```",
        (comp as any).structure || "// See full spec for details",
        "```",
        ""
      );
    }
  }

  if (spec.validation_checklist) {
    lines.push(
      "## ✅ Validation Checklist",
      ...spec.validation_checklist.map((c) => `- [ ] ${c}`),
      ""
    );
  }

  return lines.join("\n");
}

function specToPrompt(spec: UxSpec): string {
  return [
    "# SECURELLM BRIDGE — UX COMPONENT SPECIFICATION",
    "",
    "## Design Identity",
    `AESTHETIC: ${spec.philosophy.tone}`,
    `PURPOSE: ${spec.philosophy.purpose}`,
    `UNFORGETTABLE: ${spec.philosophy.memorable_element}`,
    "",
    "## Typography",
    `DISPLAY: ${spec.typography.display_font} (headers, nav)`,
    `MONO: ${spec.typography.mono_font} (data, code, metrics)`,
    `BODY: ${spec.typography.body_font} (long text only)`,
    ...((spec.typography.rules as string[]) || []).map((r) => `RULE: ${r}`),
    "",
    "## Colors (DARK MODE ONLY)",
    ...Object.entries(spec.color_system.base as Record<string, string>).map(
      ([k, v]) => `--${k}: ${v}`
    ),
    ...Object.entries(spec.color_system.accents as Record<string, string>).map(
      ([k, v]) => `--accent-${k}: ${v}`
    ),
    ...Object.entries(spec.color_system.semantic as Record<string, string>).map(
      ([k, v]) => `--semantic-${k}: ${v}`
    ),
    "",
    "## Layout",
    `GRID: ${(spec.layout_rules.grid as any).columns}-col, ${(spec.layout_rules.grid as any).gutter} gutters`,
    `SPACING: ${(spec.layout_rules.spacing as any).section_margin} sections, ${(spec.layout_rules.spacing as any).card_padding} cards`,
    `DIRECTION: ${spec.layout_rules.direction}`,
    "",
    "## Animation",
    `LOAD: ${(spec.animation as any).page_load.strategy} with ${(spec.animation as any).page_load.stagger}s stagger`,
    `HOVER: ${(spec.animation as any).interactions.hover_effect} over ${(spec.animation as any).interactions.hover_duration}ms`,
    `UPDATES: ${(spec.animation as any).data_updates.number_transition} for counters`,
    "",
    "## Tech Stack",
    ...Object.entries(spec.constraints).map(([k, v]) => {
      if (typeof v === "object") return `${k}: ${JSON.stringify(v)}`;
      return `${k}: ${v}`;
    }),
    "",
    "## ANTI-PATTERNS (DO NOT DO)",
    ...spec.anti_patterns.map((a) => `❌ ${a}`),
    "",
    "---",
    "IMPORTANT: Follow this specification EXACTLY.",
    "Do not use generic AI aesthetics. Do not default to Inter/Roboto.",
    "Use the specified color palette precisely. Dark mode is mandatory.",
    "Generate production-ready React + Tailwind code.",
    "Include TypeScript types. Use Framer Motion for animations.",
  ].join("\n");
}

function specToPromptForComponent(
  spec: UxSpec,
  component: string,
  requirements?: string
): string {
  const compDef = spec.components?.[component];
  const basePrompt = specToPrompt(spec);

  let componentSection = `\n\n## COMPONENT TO BUILD: ${component}\n`;
  if (compDef) {
    componentSection += `\n${(compDef as any).description}\n`;
    if ((compDef as any).structure) {
      componentSection += `\nLAYOUT:\n${(compDef as any).structure}\n`;
    }
    if ((compDef as any).states) {
      componentSection += `\nSTATES:\n${Object.entries((compDef as any).states)
        .map(([s, d]) => `  ${s}: ${d}`)
        .join("\n")}\n`;
    }
  }

  if (requirements) {
    componentSection += `\nADDITIONAL REQUIREMENTS:\n${requirements}\n`;
  }

  componentSection += [
    "",
    "## OUTPUT EXPECTED",
    "1. Complete React + TypeScript component file",
    "2. Tailwind CSS classes (no inline styles)",
    "3. Framer Motion animations where specified",
    "4. TypeScript interfaces for all props",
    "5. Responsive design (mobile-first)",
    "6. WCAG AA accessible",
    "",
    "Generate production-ready code now.",
  ].join("\n");

  return basePrompt + componentSection;
}

function validateComponent(
  spec: UxSpec,
  componentType: string,
  code: string
): string {
  const results: string[] = [];
  const colors = spec.color_system;

  // Check 1: Dark background
  const bgPrimary = (colors.base as any).bg_primary as string;
  if (code.includes(bgPrimary)) {
    results.push("✅ Background color matches spec");
  } else {
    results.push(`⚠️  Background color ${bgPrimary} not found in code`);
  }

  // Check 2: Display font
  const displayFont = spec.typography.display_font as string;
  if (code.includes(displayFont)) {
    results.push(`✅ Display font "${displayFont}" found`);
  } else {
    results.push(`⚠️  Display font "${displayFont}" not found — may default to Inter`);
  }

  // Check 3: Mono font for data
  const monoFont = spec.typography.mono_font as string;
  if (code.includes(monoFont)) {
    results.push(`✅ Mono font "${monoFont}" found`);
  } else {
    results.push(`⚠️  Mono font "${monoFont}" not found — metrics need monospace`);
  }

  // Check 4: Accent color used
  const accentPrimary = (colors.accents as any).primary as string;
  if (code.includes(accentPrimary)) {
    results.push(`✅ Accent color "${accentPrimary}" used`);
  } else {
    results.push(`⚠️  Accent color "${accentPrimary}" not found`);
  }

  // Check 5: Anti-patterns
  for (const anti of spec.anti_patterns) {
    const keyword = anti.split(" ").slice(0, 3).join(" ").toLowerCase();
    if (code.toLowerCase().includes(keyword)) {
      results.push(`🔴 ANTI-PATTERN DETECTED: "${anti}"`);
    }
  }

  // Check 6: Component-specific structure
  const comp = spec.components?.[componentType];
  if (comp) {
    results.push(`📋 Validating against ${componentType} spec:`);
    results.push(`  Expected: ${(comp as any).description}`);
    if ((comp as any).layout) {
      results.push(`  Layout: ${JSON.stringify((comp as any).layout)}`);
    }
  }

  results.push("", "---", "Validation complete. Fix warnings before proceeding.");

  return results.join("\n");
}

// ── Design System Reference ─────────────────────────────────────────

async function getDesignSystem(section: string): Promise<string> {
  const spec = await loadSpec("bridge-gateway");

  switch (section) {
    case "colors":
      return [
        "# Bridge Design System — Colors",
        "",
        "## Base",
        ...Object.entries(spec.color_system.base as Record<string, string>).map(
          ([k, v]) => `- \`--${k}\`: \`${v}\``
        ),
        "",
        "## Accents",
        ...Object.entries(spec.color_system.accents as Record<string, string>).map(
          ([k, v]) => `- \`--accent-${k}\`: \`${v}\``
        ),
        "",
        "## Semantic",
        ...Object.entries(spec.color_system.semantic as Record<string, string>).map(
          ([k, v]) => `- \`--semantic-${k}\`: \`${v}\``
        ),
        "",
        "## Provider Brands",
        ...Object.entries(
          (spec.color_system as any).provider_brands as Record<string, string>
        ).map(([k, v]) => `- \`${k}\`: \`${v}\``),
        "",
        "## Tailwind Config",
        "```js",
        "colors: {",
        "  bridge: {",
        `    primary: '${(spec.color_system.accents as any).primary}',`,
        `    secondary: '${(spec.color_system.accents as any).secondary}',`,
        `    highlight: '${(spec.color_system.accents as any).highlight}',`,
        "  }",
        "}",
        "```",
      ].join("\n");

    case "typography":
      return [
        "# Bridge Design System — Typography",
        "",
        "## Font Stack",
        `- Display: \`${spec.typography.display_font}\``,
        `- Monospace: \`${spec.typography.mono_font}\``,
        `- Body: \`${spec.typography.body_font}\``,
        "",
        "## Scale",
        `Ratio: ${spec.typography.scale_ratio} (Major Third)`,
        `Weights: ${(spec.typography.weights as number[]).join(", ")}`,
        "",
        "## Rules",
        ...((spec.typography.rules as string[]) || []).map((r) => `- ${r}`),
        "",
        "## Tailwind Config",
        "```js",
        "fontFamily: {",
        `  display: ['${spec.typography.display_font}', 'sans-serif'],`,
        `  mono: ['${spec.typography.mono_font}', 'monospace'],`,
        `  body: ['${spec.typography.body_font}', 'sans-serif'],`,
        "}",
        "```",
      ].join("\n");

    case "layout":
      return JSON.stringify(spec.layout_rules, null, 2);

    case "animation":
      return JSON.stringify(spec.animation, null, 2);

    case "components":
      return specToMarkdown(spec);

    case "all":
    default:
      return specToMarkdown(spec);
  }
}

// ── Spec Template Generator ─────────────────────────────────────────

function generateSpecTemplate(
  name: string,
  purpose: string,
  tone: string
): string {
  return `# ${name} — UX Specification
name: ${name}
version: 0.1.0
extends: bridge-gateway

philosophy:
  tone: "${tone || "industrial precision"}"
  purpose: "${purpose}"
  memorable_element: "Describe the one unforgettable visual element"
  target_audience:
    - "Primary user persona"

typography:
  display_font: "Space Grotesk"
  mono_font: "JetBrains Mono"
  body_font: "Inter Variable"
  scale_ratio: 1.25
  weights: [400, 500, 600, 700]
  rules:
    - "Monospace for all data and metrics"

color_system:
  mode: "dark"
  base:
    bg_primary: "#0A0E1A"
    bg_secondary: "#111827"
    bg_tertiary: "#1A2236"
  text:
    primary: "#E2E8F0"
    secondary: "#94A3B8"
    tertiary: "#64748B"
  accents:
    primary: "#6366F1"
    secondary: "#06B6D4"
  semantic:
    online: "#22C55E"
    degraded: "#F59E0B"
    offline: "#EF4444"

layout_rules:
  grid:
    columns: 12
    gutter: "24px"
  asymmetry: true
  direction: "left-heavy"
  spacing:
    section_margin: "3rem"
    card_padding: "1.5rem"
    element_gap: "1rem"

animation:
  page_load:
    strategy: "staggered_reveal"
    base_delay: 0.2
    stagger: 0.08
  interactions:
    hover_duration: 200
    hover_effect: "glow + scale(1.01)"

constraints:
  framework: "React 18+"
  styling: "Tailwind CSS v3+"
  animation_lib: "Framer Motion"
  state: "Zustand"
  icons: "Lucide React"
  accessibility: "WCAG AA"

anti_patterns:
  - "generic card grids"
  - "low-contrast color schemes"

components:
  # Define your components here following bridge-gateway patterns

validation_checklist:
  - "Aesthetic is distinctive"
  - "Colors match spec"
  - "Fonts match spec"
  - "Animations are polished"
  - "Responsive at all breakpoints"
  - "WCAG AA compliant"
`;
}

// ── Main Handler ────────────────────────────────────────────────────

export async function handleUxTool(
  toolName: string,
  args: Record<string, unknown>
): Promise<string> {
  switch (toolName) {
    case "ux_list_specs": {
      const specs = await listSpecFiles();
      return JSON.stringify({ specs, count: specs.length }, null, 2);
    }

    case "ux_get_spec": {
      const { spec_name, format = "markdown" } = args as {
        spec_name: string;
        format?: string;
      };
      const spec = await loadSpec(spec_name);

      switch (format) {
        case "yaml":
          return await readFile(
            join(UX_SPECS_DIR, `${spec_name}.yml`),
            "utf-8"
          );
        case "json":
          return JSON.stringify(spec, null, 2);
        case "prompt":
          return specToPrompt(spec);
        case "markdown":
        default:
          return specToMarkdown(spec);
      }
    }

    case "ux_generate_prompt": {
      const { spec_name, component, requirements } = args as {
        spec_name: string;
        component: string;
        requirements?: string;
      };
      const spec = await loadSpec(spec_name);
      return specToPromptForComponent(spec, component, requirements);
    }

    case "ux_validate_component": {
      const { spec_name, component_type, code } = args as {
        spec_name: string;
        component_type: string;
        code: string;
      };
      const spec = await loadSpec(spec_name);
      return validateComponent(spec, component_type, code);
    }

    case "ux_design_system": {
      const { section = "all" } = args as { section?: string };
      return await getDesignSystem(section);
    }

    case "ux_create_spec": {
      const { name, purpose, tone = "industrial precision" } = args as {
        name: string;
        purpose: string;
        tone?: string;
      };
      return generateSpecTemplate(name, purpose, tone);
    }

    default:
      throw new Error(`Unknown UX tool: ${toolName}`);
  }
}
