export const meta = {
  name: 'reasonix-task',
  description: 'Run a custom coding task through Reasonix (DeepSeek) — refactor, generate, analyze, or debug code',
  phases: [
    { title: 'Context', detail: 'Gathering project context' },
    { title: 'Reasonix Execution', detail: 'Sending task to Reasonix' },
    { title: 'Synthesis', detail: 'Processing Reasonix output' },
  ],
}

const TASK_OUTPUT_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string', description: 'What Reasonix did and the key results' },
    filesChanged: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          path: { type: 'string' },
          action: { type: 'string', enum: ['created', 'modified', 'deleted', 'analyzed'] },
          description: { type: 'string' },
        },
        required: ['path', 'action', 'description'],
      },
    },
    keyDecisions: {
      type: 'array',
      items: { type: 'string' },
      description: 'Important design or implementation decisions made',
    },
    concerns: {
      type: 'array',
      items: { type: 'string' },
      description: 'Any concerns, trade-offs, or things to verify',
    },
    recommendations: {
      type: 'array',
      items: { type: 'string' },
      description: 'Next steps or recommendations',
    },
  },
  required: ['summary', 'filesChanged'],
}

// ─── Phase 1: Gather project context ────────────────────────────────────────

phase('Context')
log('Gathering project context...')

const projectContext = await agent(
  `You are a project context gatherer. Explore the current project and return structured info about it.

Run these commands:
1. Check what kind of project this is (package.json, Cargo.toml, pyproject.toml, go.mod, etc.)
2. List the top-level directory structure
3. List the src/ or lib/ directory structure (2 levels deep)
4. Check git branch and recent commits (\`git log --oneline -5\`)
5. Check if there's a CLAUDE.md or README.md with project info

Return ALL data — do not analyze or modify anything.`,
  {
    label: 'project-context',
    phase: 'Context',
    schema: {
      type: 'object',
      properties: {
        projectType: { type: 'string' },
        language: { type: 'string' },
        framework: { type: 'string' },
        projectStructure: { type: 'string' },
        currentBranch: { type: 'string' },
        recentCommits: { type: 'string' },
        hasReadme: { type: 'boolean' },
        readmeSnippet: { type: 'string' },
      },
      required: ['projectType', 'language', 'projectStructure', 'currentBranch'],
    },
  },
)

if (!projectContext) {
  log('Could not gather project context — proceeding without it')
}

log(`Project: ${projectContext?.projectType || 'unknown'}, branch: ${projectContext?.currentBranch || 'unknown'}`)

// ─── Phase 2: Send task to Reasonix ─────────────────────────────────────────

phase('Reasonix Execution')
log('Sending task to Reasonix...')

// args contains whatever the user passes when invoking the workflow
const userTask = typeof args === 'string' ? args
  : args?.task || args?.prompt || JSON.stringify(args) || 'No task specified'

const reasonixPrompt = `You are a coding agent powered by Reasonix (DeepSeek).

## Project Context
- Type: ${projectContext?.projectType || 'unknown'}
- Language: ${projectContext?.language || 'unknown'}
- Framework: ${projectContext?.framework || 'unknown'}
- Branch: ${projectContext?.currentBranch || 'unknown'}
- Structure: ${projectContext?.projectStructure || 'unknown'}

## Task
${userTask}

Execute this task using Reasonix. Use the reasonix_run MCP tool if available to process the task.

Steps:
1. First read any relevant files to understand context
2. Execute the task using Reasonix (call reasonix_run with the appropriate prompt)
3. Verify the output is correct
4. Report what was done

Be thorough and precise. If the task involves code changes, apply them directly.`

const result = await agent(reasonixPrompt, {
  label: 'reasonix-task',
  phase: 'Reasonix Execution',
  schema: TASK_OUTPUT_SCHEMA,
})

if (!result) {
  log('Reasonix task was skipped or failed')
  return { error: 'Reasonix did not complete the task' }
}

// ─── Phase 3: Synthesize ────────────────────────────────────────────────────

phase('Synthesis')
log(`Task complete — ${result.filesChanged.length} files affected`)

// If the task affected files, verify they exist and are reasonable
if (result.filesChanged.length > 0) {
  const createdFiles = result.filesChanged.filter(f => f.action === 'created' || f.action === 'modified')
  if (createdFiles.length > 0) {
    log(`Files affected: ${createdFiles.map(f => f.path).join(', ')}`)
  }
}

return result
