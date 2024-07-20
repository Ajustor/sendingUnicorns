import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete"
import { ViewPlugin, MatchDecorator, Decoration, hoverTooltip } from '@codemirror/view'
import Mustache from 'mustache'


export const defineCodeMirrorCompletion = (variables?: [string, string][] | null) => {
  return function (context: CompletionContext): CompletionResult | null {

    let word = context.matchBefore(/\{\{\w*/)
    if (word?.from === word?.to && !context.explicit || !variables) {
      return null
    }

    return {
      from: word?.from ?? context.pos,
      options: variables.map(([key, value]) => ({ label: `{{${key}}}`, info: value })),
    }
  }
}

export const defineHover = (variables?: [string, string][] | null) => {
  return hoverTooltip((view, pos, side) => {
    const { from, to, text } = view.state.doc.lineAt(pos)
    let start = pos, end = pos
    while (start > from && /\w/.test(text[start - from - 1])) { start-- }
    while (end < to && /\w/.test(text[end - from])) { end++ }
    if (start == pos && side < 0 || end == pos && side > 0 || !variables) {
      return null
    }

    return {
      pos: start,
      end,
      above: true,
      arrow: true,
      create(view) {
        const dom = document.createElement("div")
        const varKey = text.slice(start - from, end - from)
        const variable = variables.find(([key]) => key === varKey)
        if (variable) {
          const parsedVariables = variables.reduce(
            (acc, [key, value]) => ({ ...acc, [key]: value }),
            {}
          )
          dom.classList.add('p-2')
          dom.textContent = Mustache.render(variable?.[1], parsedVariables, {}, { escape: (s: string) => s })
        }
        dom.textContent ??= ''
        return { dom }
      }
    }
  })
}

export const defineCodeMirrorLanguage = (variables?: [string, string][] | null) => {
  const decorator = new MatchDecorator({
    regexp: /\{\{([\w]+)\}\}/g,
    decoration: (match) => {
      const found = variables?.find(([key]) => key === match[1])
      if (found) {
        return Decoration.mark({ class: "valid" })
      }

      return Decoration.mark({ class: "invalid" })
    }
  })
  return ViewPlugin.define(view => ({
    decorations: decorator.createDeco(view),
    update(u) { this.decorations = decorator.updateDeco(u, this.decorations) }
  }), {
    decorations: v => v.decorations
  })
}