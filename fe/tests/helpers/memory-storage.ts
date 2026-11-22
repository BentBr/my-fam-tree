/** In-memory `Storage` for stubbing `localStorage` / `sessionStorage` in tests. */
export function memoryStorage(): Storage {
    const store = new Map<string, string>()
    return {
        getItem: (key: string) => store.get(key) ?? null,
        setItem: (key: string, value: string) => {
            store.set(key, value)
        },
        removeItem: (key: string) => {
            store.delete(key)
        },
        clear: () => {
            store.clear()
        },
        key: (index: number) => [...store.keys()][index] ?? null,
        get length() {
            return store.size
        },
    }
}
