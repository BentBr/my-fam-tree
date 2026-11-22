// Colours belong in src/design-system (tokens / theme); everywhere else must reference them.
const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i
const COLOR_FN = /^(rgb|rgba|hsl|hsla)\(/i

function isRawColor(text) {
    const value = text.trim()
    return HEX.test(value) || COLOR_FN.test(value)
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
        return {
            Literal(node) {
                if (typeof node.value === 'string' && isRawColor(node.value)) {
                    context.report({ node, messageId: 'rawColor', data: { value: node.value } })
                }
            },
            TemplateElement(node) {
                const text = node.value.cooked ?? node.value.raw
                if (isRawColor(text)) {
                    context.report({ node, messageId: 'rawColor', data: { value: text } })
                }
            },
        }
    },
}
