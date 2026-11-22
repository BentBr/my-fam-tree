// Colours belong in src/design-system (tokens / theme); everywhere else must reference them.
const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i
// `rgb(var(--v-theme-…))` reads a theme token (Vuetify stores theme colours as RGB triplets), so it is allowed.
const COLOR_FN = /^(rgb|rgba|hsl|hsla)\(\s*(?!var\()/i
// Unanchored variant for CSS text (style attributes and <style> blocks).
const CSS_COLOR = /(?<![\w#-])#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})(?![\w-])|\b(?:rgba?|hsla?)\(\s*(?!var\()/gi

function isRawColor(text) {
    const value = text.trim()
    return HEX.test(value) || COLOR_FN.test(value)
}

/** Replace CSS comments with spaces of the same shape so match offsets stay valid. */
function blankCssComments(text) {
    return text.replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, ' '))
}

/** @type {import('eslint').Rule.RuleModule} */
export default {
    meta: {
        type: 'problem',
        docs: { description: 'Disallow raw colour literals outside the design system' },
        messages: {
            rawColor: 'Raw colour "{{ value }}": use a design-system token (src/design-system) instead.',
        },
        schema: [],
    },
    create(context) {
        const { sourceCode } = context

        function reportNode(node, value) {
            context.report({ node, messageId: 'rawColor', data: { value } })
        }

        /** Report every colour inside CSS `text` that starts at source offset `offset`. */
        function reportCss(text, offset) {
            for (const match of blankCssComments(text).matchAll(CSS_COLOR)) {
                const start = offset + match.index
                context.report({
                    loc: {
                        start: sourceCode.getLocFromIndex(start),
                        end: sourceCode.getLocFromIndex(start + match[0].length),
                    },
                    messageId: 'rawColor',
                    data: { value: match[0] },
                })
            }
        }

        const scriptVisitor = {
            Literal(node) {
                if (typeof node.value === 'string' && isRawColor(node.value)) reportNode(node, node.value)
            },
            TemplateElement(node) {
                const text = node.value.cooked ?? node.value.raw
                if (isRawColor(text)) reportNode(node, text)
            },
        }

        const services = sourceCode.parserServices
        if (typeof services?.defineTemplateBodyVisitor !== 'function') return scriptVisitor

        const templateVisitor = {
            ...scriptVisitor,
            'VAttribute[directive=false]'(node) {
                const { value } = node
                if (value === null) return
                if (node.key.name === 'style') {
                    const quoted = /^["']/.test(sourceCode.text.charAt(value.range[0]))
                    reportCss(value.value, value.range[0] + (quoted ? 1 : 0))
                } else if (isRawColor(value.value)) {
                    reportNode(value, value.value)
                }
            },
        }

        const styleVisitor = {
            'Program:exit'() {
                const fragment = services.getDocumentFragment?.()
                for (const element of fragment?.children ?? []) {
                    if (element.type !== 'VElement' || element.name !== 'style') continue
                    for (const child of element.children) {
                        if (child.type === 'VText') reportCss(child.value, child.range[0])
                    }
                }
            },
        }

        return services.defineTemplateBodyVisitor(templateVisitor, { ...scriptVisitor, ...styleVisitor })
    },
}
