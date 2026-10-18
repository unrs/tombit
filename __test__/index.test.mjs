import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

import { format, lint, version } from '../index.js'

// Keep schema lookup off so no test reaches the network.
const OFFLINE = { schema: { enabled: false } }

const INDENTED =
  '[schema]\nenabled = false\n\n[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 4\n'

describe('tombit', () => {
  it('exposes the package version', () => {
    expect(version()).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('formats TOML', () => {
    const result = format('a=1\n', 'a.toml', { overrideConfig: OFFLINE })

    expect(result.diagnostics).toEqual([])
    expect(result.formatted).toBe('a = 1\n')
  })

  it('applies the override config', () => {
    const result = format('[table]\nkey = 1\n', 'a.toml', {
      overrideConfig: {
        ...OFFLINE,
        format: {
          rules: { 'indent-table-key-value-pairs': true, 'indent-width': 4 },
        },
      },
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('discovers tombi.toml from the file directory', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      overrideConfig: OFFLINE,
    })

    expect(result.formatted).toBe('[table]\n      key = 1\n')
  })

  it('uses a supplied config instead of discovering one', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      config: INDENTED,
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('accepts a config file object', () => {
    const result = format('[table]\nkey = 1\n', 'a.toml', {
      config: { content: INDENTED, path: 'tombi.toml' },
    })

    expect(result.formatted).toBe('[table]\n    key = 1\n')
  })

  it('skips discovery when config is false', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tombit-'))
    fs.writeFileSync(
      path.join(dir, 'tombi.toml'),
      '[format.rules]\nindent-table-key-value-pairs = true\nindent-width = 6\n',
    )

    const result = format('[table]\nkey = 1\n', path.join(dir, 'a.toml'), {
      config: false,
    })

    expect(result.formatted).toBe('[table]\nkey = 1\n')
  })

  it('rejects config: true', () => {
    expect(() => format('a=1\n', 'a.toml', { config: true })).toThrow(
      /not meaningful/,
    )
  })

  it('rejects unknown override keys', () => {
    expect(() =>
      format('a=1\n', 'a.toml', { overrideConfig: { nosuchkey: true } }),
    ).toThrow(/nosuchkey/)
  })

  it('reports lint diagnostics', () => {
    const result = lint('invalid = =\n', 'a.toml', { overrideConfig: OFFLINE })

    expect(result.diagnostics.length).toBeGreaterThan(0)
    expect(result.diagnostics[0].level).toBe('error')
  })
})
