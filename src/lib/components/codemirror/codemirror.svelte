<script lang="ts">
  import { EditorState } from '@codemirror/state'
  import { autocompletion } from '@codemirror/autocomplete'
  import {
    EditorView,
    type EditorViewConfig,
    placeholder as placeholderExt,
    keymap,
    lineNumbers
  } from '@codemirror/view'
  import { LanguageSupport } from '@codemirror/language'
  import { defineCodeMirrorCompletion, defineCodeMirrorLanguage, defineHover } from './extensions'
  import { mode } from 'mode-watcher'
  import { markdown } from '@codemirror/lang-markdown'

  type Props = {
    value: string
    placeholder: string
    class?: string
    variables: [string, string][]
    onkeyup?: () => void
    onCtrlS?: () => boolean
    isSingleLine?: boolean
    language?: LanguageSupport
  } & EditorViewConfig

  let {
    value = $bindable(),
    placeholder,
    class: className,
    variables,
    onkeyup = () => {},
    onCtrlS = () => true,
    isSingleLine = true,
    language = markdown(),
    ...options
  }: Props = $props()

  function getExtensions(localVariables: [string, string][]) {
    const extensions = [
      language,
      keymap.of([
        {
          key: 'Mod-s',
          run: onCtrlS
        }
      ]),
      placeholderExt(placeholder),
      EditorView.theme({
        '.valid': {
          color: 'green'
        },
        '.invalid': {
          color: 'red'
        },
        '&': {
          width: '100%',
          'padding-left': '0.75rem',
          'padding-right': '0.75rem',
          'padding-top': '0.5rem',
          'padding-bottom': '0.5rem',
          'font-size': '0.875rem' /* 14px */,
          'line-height': '1.25rem' /* 20px */
        },
        '.cm-content': {
          'caret-color': $mode === 'light' ? 'black' : 'white'
        },
        '.cm-tooltip': {
          'border-radius': '0.5rem',
          'background-color': 'unset'
        }
      })
    ]

    if (localVariables.length) {
      extensions.push(
        defineCodeMirrorLanguage(localVariables),
        autocompletion({
          defaultKeymap: true,
          activateOnTyping: true,
          override: [defineCodeMirrorCompletion(localVariables)]
        }),
        defineHover(localVariables)
      )
    }

    if (isSingleLine) {
      extensions.push(
        EditorState.transactionFilter.of((transaction) => {
          return transaction.newDoc.lines > 1
            ? [
                transaction,
                {
                  changes: {
                    from: 0,
                    to: transaction.newDoc.length,
                    insert: transaction.newDoc.sliceString(0, undefined, '')
                  },
                  sequential: true
                }
              ]
            : [transaction]
        })
      )
    } else {
      extensions.push(lineNumbers())
    }

    return extensions
  }

  const getState = (doc: string) =>
    EditorState.create({
      doc,
      // selection: {
      //   anchor: doc.length
      // },
      extensions: getExtensions(variables)
    })

  let currentCursor: number = 0

  function setCursor(pos: number) {
    if (typeof editor !== 'undefined') {
      currentCursor = pos
      editor.dispatch({ selection: { anchor: currentCursor } })
    }
  }

  function getCursor() {
    if (typeof editor !== 'undefined') {
      currentCursor = editor.state.selection.main.head
      return currentCursor
    } else {
      return 0
    }
  }

  let editorElement: HTMLElement
  let editor: EditorView

  $effect(() => {
    if (editor) {
      // editor.destroy()
      const cursorPos = getCursor()
      editor.setState(getState(value))
      setCursor(cursorPos)
    } else {
      editor = new EditorView({
        state: getState(value),
        ...options,
        parent: editorElement,
        dispatch(transaction) {
          editor.update([transaction])

          value = editor.state.doc.toString()
          onkeyup()
        }
      })
    }
  })
</script>

<span class="code-editor {className}" bind:this={editorElement}></span>

<style>
  .code-editor,
  .cm-scroller {
    @apply flex w-full rounded-md border border-input bg-background ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50;
  }

  .cm-editor.valid {
    color: purple;
  }

  .invalid {
    color: red;
  }
</style>
