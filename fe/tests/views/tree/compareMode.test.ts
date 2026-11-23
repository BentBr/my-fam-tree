import { describe, expect, it } from 'vitest'

import { knownPair, nextComparePair, parseCompareParam, withTreeQuery } from '@/views/tree/compareMode'

const A = '00000000-0000-4000-8000-00000000000a'
const B = '00000000-0000-4000-8000-00000000000b'
const C = '00000000-0000-4000-8000-00000000000c'

describe('parseCompareParam', () => {
    it('reads one or two ids', () => {
        expect(parseCompareParam(A)).toEqual([A])
        expect(parseCompareParam(`${A},${B}`)).toEqual([A, B])
    })

    it('drops malformed, duplicate and surplus ids', () => {
        expect(parseCompareParam(undefined)).toEqual([])
        expect(parseCompareParam(['x'])).toEqual([])
        expect(parseCompareParam(`nope,${A}`)).toEqual([A])
        expect(parseCompareParam(`${A},${A}`)).toEqual([A])
        expect(parseCompareParam(`${A},${B},${C}`)).toEqual([A, B])
    })

    it('lower-cases ids before comparing them', () => {
        expect(parseCompareParam(`${A.toUpperCase()},${A}`)).toEqual([A])
        expect(parseCompareParam(B.toUpperCase())).toEqual([B])
    })
})

describe('knownPair', () => {
    it('is empty while the tree is unknown', () => {
        expect(knownPair([A, B], null)).toEqual([])
    })

    it('keeps only ids of the loaded tree', () => {
        expect(knownPair([A, B], new Set([A, B]))).toEqual([A, B])
        expect(knownPair([A, B], new Set([B]))).toEqual([B])
        expect(knownPair([A], new Set())).toEqual([])
    })
})

describe('nextComparePair', () => {
    it('fills A, then B, then replaces B', () => {
        expect(nextComparePair([], A)).toEqual([A])
        expect(nextComparePair([A], B)).toEqual([A, B])
        expect(nextComparePair([A, B], C)).toEqual([A, C])
    })

    it('deselects a selected person; the survivor moves to slot A', () => {
        expect(nextComparePair([A, B], A)).toEqual([B])
        expect(nextComparePair([A, B], B)).toEqual([A])
        expect(nextComparePair([A], A)).toEqual([])
    })
})

describe('withTreeQuery', () => {
    it('applies center and compare in one query and keeps unrelated params', () => {
        const next = withTreeQuery({ center: A, compare: `${A},${B}`, x: '1' }, { center: null, compare: [] })
        expect(next).toEqual({ x: '1' })
    })

    it('leaves a param alone when the patch does not mention it', () => {
        expect(withTreeQuery({ center: A }, { compare: [A, B] })).toEqual({ center: A, compare: `${A},${B}` })
        expect(withTreeQuery({ compare: A }, { center: B })).toEqual({ compare: A, center: B })
    })
})
