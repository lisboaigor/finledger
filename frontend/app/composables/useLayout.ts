import { computed, reactive } from 'vue'

const THEME_COOKIE = 'finledger_theme'
const ONE_YEAR = 60 * 60 * 24 * 365

const layoutState = reactive({
    darkTheme: false,
    /** Sidebar recolhida no desktop (o usuário escondeu manualmente). */
    staticMenuInactive: false,
    /** Drawer da sidebar aberto no mobile. */
    mobileMenuActive: false,
})

let themeInitialized = false

export function useLayout() {
    // Persisted theme choice. A cookie (not localStorage) so the server can
    // render the `app-dark` class on <html> and reloads don't flash/reset.
    const themeCookie = useCookie<string | null>(THEME_COOKIE, { maxAge: ONE_YEAR })

    if (!themeInitialized) {
        layoutState.darkTheme = themeCookie.value === 'dark'
        themeInitialized = true
    }

    /** Sets the theme. `persist: false` applies it without touching the saved
     * preference (used by the PDV, which manages its own default). */
    const setDarkTheme = (dark: boolean, opts: { persist?: boolean } = {}) => {
        layoutState.darkTheme = dark
        if (opts.persist !== false) themeCookie.value = dark ? 'dark' : 'light'
    }

    /** Re-applies whatever theme is saved in the cookie (PDV exit path). */
    const restorePersistedTheme = () => {
        layoutState.darkTheme = themeCookie.value === 'dark'
    }

    const toggleDarkMode = () => {
        if (typeof document === 'undefined' || !document.startViewTransition) {
            setDarkTheme(!layoutState.darkTheme)
            return
        }
        document.startViewTransition(() => setDarkTheme(!layoutState.darkTheme))
    }

    const isDesktop = () => typeof window !== 'undefined' && window.innerWidth >= 1024

    const toggleMenu = () => {
        if (isDesktop()) {
            layoutState.staticMenuInactive = !layoutState.staticMenuInactive
        } else {
            layoutState.mobileMenuActive = !layoutState.mobileMenuActive
        }
    }

    const hideMobileMenu = () => {
        layoutState.mobileMenuActive = false
    }

    const isDarkTheme = computed(() => layoutState.darkTheme)

    return {
        layoutState,
        isDarkTheme,
        toggleDarkMode,
        setDarkTheme,
        restorePersistedTheme,
        toggleMenu,
        hideMobileMenu,
        isDesktop,
    }
}
