<script lang="ts">
  import { EditorState } from '@codemirror/state'
  import { autocompletion, completionKeymap, currentCompletions } from '@codemirror/autocomplete'
  import {
    EditorView,
    type EditorViewConfig,
    placeholder as placeholderExt
  } from '@codemirror/view'
  import { defineCodeMirrorCompletion, defineCodeMirrorLanguage, defineHover } from './extensions'
  import { minimalSetup } from 'codemirror'

  type Props = {
    value: string
    placeholder: string
    class?: string
    variables: [string, string][]
    onkeyup?: () => void
  } & EditorViewConfig

  let {
    value = $bindable(),
    placeholder,
    class: className,
    variables,
    onkeyup = () => {},
    ...options
  }: Props = $props()

  const getState = (doc: string) =>
    EditorState.create({
      doc,
      selection: {
        anchor: doc.length
      },
      extensions: [
        minimalSetup,
        placeholderExt(placeholder),
        defineCodeMirrorLanguage(variables),
        autocompletion({
          defaultKeymap: true,
          activateOnTyping: true,
          override: [defineCodeMirrorCompletion(variables)]
        }),
        defineHover(variables),
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
        }),
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
          '.cm-scroller': {
            'overflow-y': 'hidden'
          },
          '.cm-tooltip': {
            'border-radius': '0.5rem'
          }
        })
      ]
    })

  let editorElement: HTMLElement
  let editor: EditorView

  $effect(() => {
    if (editor) {
      // editor.destroy()
      editor.setState(getState(value))
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
    @apply flex h-10 w-full rounded-md border border-input bg-background ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50;
  }

  .cm-editor.valid {
    color: purple;
  }

  .invalid {
    color: red;
  }
</style>
