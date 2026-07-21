import tailwindcss from '@tailwindcss/vite'

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },

  app: {
    head: {
      // Coruja da marca (mesmo asset do AppLogoIcon); o favicon.ico fica como fallback.
      // Grand Hotel: script do wordmark (.brand-wordmark); Bricolage Grotesque: display do hero.
      link: [
        // Coruja Icons8 (Forma Light) tingida na cor primária da marca (#1AA886).
        // O ?v= força rebusca quando a arte muda: o cache de favicon do
        // navegador ignora hard-refresh.
        {
          rel: 'icon',
          type: 'image/png',
          sizes: '32x32',
          href: '/icons8-owl-forma-light-32.png?v=5',
        },
        { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
        { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' },
        {
          rel: 'stylesheet',
          // Grand Hotel + Bricolage: marca padrão. Pacifico/Playfair/Poppins/
          // Montserrat: opções do seletor de fonte do wordmark whitelabel
          // (o navegador só baixa o arquivo da fonte de fato usada). Inter vem
          // do @import em tailwind.css.
          href: 'https://fonts.googleapis.com/css2?family=Grand+Hotel&family=Bricolage+Grotesque:opsz,wght@12..96,600..800&family=Pacifico&family=Playfair+Display:wght@600;700&family=Poppins:wght@600;700&family=Montserrat:wght@600;700&display=swap',
        },
      ],
    },
  },

  modules: ['shadcn-nuxt'],

  shadcn: {
    // Sem prefixo: <Button>, <Dialog> etc. resolvem direto para app/components/ui.
    prefix: '',
    componentDir: './app/components/ui',
  },

  imports: {
    dirs: ['composables/viewmodels'],
  },

  css: ['~/assets/css/tailwind.css'],

  vite: {
    plugins: [tailwindcss()],
  },

  devServer: {
    port: 3001,
  },

  nitro: {
    devProxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },

  runtimeConfig: {
    public: {
      apiBase: '/api',
      // Domínio-base dos tenants em produção (NUXT_PUBLIC_BASE_DOMAIN, ex.:
      // finledger.com.br). Vazio em dev — cai na heurística *.localhost.
      baseDomain: '',
    },
  },
})
