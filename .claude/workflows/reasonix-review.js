export const meta = {
  name: 'reasonix-review',
  description: 'Review current code changes (git diff) using Reasonix — gives a second-opinion review from DeepSeek',
  phases: [
    { title: 'Context', detail: 'Gathering git diff and changed files' },
    { title: 'Reasonix Review', detail: 'Sending changes to Reasonix for deep review' },
  ],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string', description: 'Overall summary of the review' },
    findings: {
      type: 'array',
      description: 'List of issues or observations found',
      items: {
        type: 'object',
        properties: {
          severity: { type: 'string', enum: ['critical', 'warning', 'suggestion'] },
          file: { type: 'string' },
          line: { type: 'number' },
          title: { type: 'string' },
          description: { type: 'string' },
          recommendation: { type: 'string' },
        },
        required: ['severity', 'file', 'title', 'description'],
      },
    },
    strengths: {
      type: 'array',
      items: { type: 'string' },
      description: 'What the code does well',
    },
    overallVerdict: {
      type: 'string',
      enum: ['approved', 'changes-requested', 'needs-discussion'],
    },
  },
  required: ['summary', 'findings', 'overallVerdict'],
}

// ─── Phase 1: Gather context ────────────────────────────────────────────────

phase('Context')
log('Getting current git diff...')

// We gather context via agent — subagents have full tool access (Bash, MCP)
const context = await agent(
  `You are a context gatherer. Run these steps and return the results:

1. \`git diff HEAD\` — get the full diff of staged+unstaged changes
2. \`git diff --stat HEAD\` — get the file summary
3. For each changed file, check if it exists and read the first 30 lines to understand context
4. Check the project type (package.json, Cargo.toml, etc.)

Return ALL the raw data — do NOT analyze or review yet. Just collect and return.`,
  {
    label: 'gather-context',
    phase: 'Context',
    schema: {
      type: 'object',
      properties: {
        diff: { type: 'string' },
        diffStat: { type: 'string' },
        projectType: { type: 'string' },
        changedFiles: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              path: { type: 'string' },
              context: { type: 'string' },
            },
            required: ['path'],
          },
        },
      },
      required: ['diff', 'diffStat', 'changedFiles'],
    },
  },
)

if (!context) {
  log('Failed to gather context — aborting')
  return { error: 'Could not gather git context' }
}

log(`Found ${context.changedFiles.length} changed files across ${context.diffStat.split('\n').length} lines of diff stats`)

// ─── Phase 2: Send to Reasonix ──────────────────────────────────────────────

phase('Reasonix Review')
log('Sending diff to Reasonix for deep review...')

const diffPreview = context.diff.length > 8000
  ? context.diff.slice(0, 8000) + '\n\n... [diff truncated to 8000 chars]'
  : context.diff

const reviewPrompt = `You are a senior code reviewer using Reasonix (DeepSeek).

Review the following git diff carefully. For each change, evaluate:
1. **Correctness** — does it introduce bugs, race conditions, or logic errors?
2. **Security** — any injection risks, unsafe data handling, or exposed secrets?
3. **Performance** — any obvious perf regressions (N+1 queries, unnecessary re-renders, etc.)?
4. **Style & maintainability** — does it match surrounding code conventions?

Be specific. Reference exact file paths and line numbers from the diff.

## Project context
- Type: ${context.projectType || 'unknown'}
- Files changed:\n${context.changedFiles.map(f => `  - ${f.path}`).join('\n')}

## Diff to review:
\`\`\`diff
${diffPreview}
\`\`\`

Return structured findings with severity, file, line, title, description, and recommendation.`

const review = await agent(reviewPrompt, {
  label: 'reasonix-review',
  phase: 'Reasonix Review',
  schema: REVIEW_SCHEMA,
})

if (!review) {
  log('Review was skipped or failed')
  return { error: 'Reasonix review did not complete' }
}

// ─── Output ─────────────────────────────────────────────────────────────────

log(`Review complete — ${review.findings.length} findings, verdict: ${review.overallVerdict}`)

return review
