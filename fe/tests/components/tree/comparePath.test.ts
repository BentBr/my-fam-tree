import { describe, expect, it } from 'vitest'

import { edgeKey, pathEdgeKeys } from '@/components/tree/comparePath'

describe('comparePath', () => {
    it('builds the same key for both orientations of an edge', () => {
        expect(edgeKey('a', 'b')).toBe(edgeKey('b', 'a'))
        expect(edgeKey('a', 'b')).not.toBe(edgeKey('a', 'c'))
    })

    it('covers only pairs of ADJACENT path members', () => {
        const keys = pathEdgeKeys(['a', 'b', 'c'])
        expect(keys.has(edgeKey('a', 'b'))).toBe(true)
        expect(keys.has(edgeKey('c', 'b'))).toBe(true)
        // a-c is a chord between two path members, not part of the path.
        expect(keys.has(edgeKey('a', 'c'))).toBe(false)
        expect(keys.size).toBe(2)
    })

    it('has no pairs for an empty or single-person path', () => {
        expect(pathEdgeKeys([]).size).toBe(0)
        expect(pathEdgeKeys(['a']).size).toBe(0)
    })
})
