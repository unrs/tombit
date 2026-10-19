import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

import { format, formatSync, lint, lintSync, version } from '../index.js'

// Keep schema lookup off so no test reaches the network.
const OFFLINE = { schema: { enabled: false } }

const INDENTED =
  '[schema]\nenabled = false\n\n[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 4\n'

describe('tombit', () => {
  it('exposes the package version', () => {
    expect(version()).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('formats TOML', async () => {
    const result = await format('a=1\n', 'a.toml', { overrideConfig: OFFLINE })

    expect(result.diagnostics).toEqual([])
    expect(result.formatted).toBe('a = 1\n')
  })

  it('formats TOML synchronously', () => {
    const result = formatSync('a=1\n', 'a.toml', { overrideConfig: OFFLINE })

    expect(result.diagnostics).toEqual([])
    expect(result.formatted).toBe('a = 1\n')
  })

  it('applies the override config', async () => {
    const result = await format('[table]\nkey = 1\n', 'a.toml', {
      overrideConfig: {
        ...OFFLINE,
        format: {
          rules: { 'indent-table-key-value-pairs': true, 'indent-width': 4 },
        },
      },
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('discovers tombi.toml from the file directory', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = await format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      overrideConfig: OFFLINE,
    })

    expect(result.formatted).toBe('[table]\n      key = 1\n')
  })

  it('uses a supplied config instead of discovering one', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = await format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      config: INDENTED,
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('accepts a config file object', async () => {
    const result = await format('[table]\nkey = 1\n', 'a.toml', {
      config: { content: INDENTED, path: 'tombi.toml' },
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('skips discovery when config is false', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = await format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      config: false,
    })

    expect(result.formatted).toBe('[table]\nkey = 1\n')
  })

  it('rejects config: true', async () => {
    await expect(format('a=1\n', 'a.toml', { config: true })).rejects.toThrow(
      /not meaningful/,
    )
  })

  it('rejects unknown override keys', async () => {
    await expect(
      format('a=1\n', 'a.toml', { overrideConfig: { nosuchkey: true } }),
    ).rejects.toThrow(/nosuchkey/)
  })

  // `[[overrides]]` rules take precedence over the base rules
  // (tombi-toml/tombi#2329, fixed upstream in tombi-toml/tombi#2330).
  it('formats a discovered config with overrides', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 4\n\n[[overrides]]\nfiles.include = ["**/*.toml"]\n\n[overrides.format.rules]\nindent-width = 6\n',
    )

    const result = await format('[t]\nkey = 1\n', path.join(dir, 'a.toml'), {
      overrideConfig: OFFLINE,
    })

    expect(result.formatted).toBe('[t]\n      key = 1\n')
  })

  it('formats a discovered config with overrides synchronously', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 4\n\n[[overrides]]\nfiles.include = ["**/*.toml"]\n\n[overrides.format.rules]\nindent-width = 6\n',
    )

    const result = formatSync('[t]\nkey = 1\n', path.join(dir, 'a.toml'))

    expect(result.formatted).toBe('[t]\n      key = 1\n')
  })

  it('reports the diagnostic range when formatting fails', async () => {
    const result = await format('[invalid\nkey = ', 'a.toml', {
      overrideConfig: OFFLINE,
    })

    expect(result.formatted).toBeUndefined()

    const [diagnostic] = result.diagnostics
    expect(diagnostic.level).toBe('error')
    expect(diagnostic.code).toBe('expected-bracket-end')
    expect(diagnostic.range.start).toEqual({ line: 0, column: 8 })
    expect(diagnostic.range.end.line).toBeGreaterThanOrEqual(0)
    expect(diagnostic.sourceFile).toBeNull()
  })

  it('reports lint diagnostics', async () => {
    const result = await lint('invalid = =\n', 'a.toml', {
      overrideConfig: OFFLINE,
    })

    expect(result.diagnostics.length).toBeGreaterThan(0)
    expect(result.diagnostics[0].level).toBe('error')
    expect(result.diagnostics[0].range.start.line).toBe(0)
    expect(result.diagnostics[0].sourceFile).toBeNull()
  })

  it('lints synchronously', () => {
    const result = lintSync('invalid = =\n', 'a.toml', {
      overrideConfig: OFFLINE,
    })

    expect(result.diagnostics.length).toBeGreaterThan(0)
  })
})
