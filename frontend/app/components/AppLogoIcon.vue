<!--
    Marca da aplicação. Sem logo personalizado (whitelabel), mostra a coruja da
    Finledger (/public/owl.png, também favicon) aplicada como CSS mask sobre
    currentColor — herda a cor do texto e o tamanho da fonte (1em) do pai, como
    um ícone do PrimeIcons: <AppLogoIcon class="text-primary" style="font-size: 1.5rem" />.
    Com logo do tenant, renderiza a imagem enviada, com altura de 1em.
-->
<template>
    <img
        v-if="logoDataUri"
        :src="logoDataUri"
        alt=""
        aria-hidden="true"
        class="app-logo-img"
    >
    <span v-else class="app-logo-icon" aria-hidden="true" />
</template>

<script setup lang="ts">
const { logoDataUri } = useMarca()
</script>

<style scoped>
.app-logo-icon {
    display: inline-block;
    width: 1em;
    height: 1em;
    background-color: currentColor;
    mask: url('/owl.png') center / contain no-repeat;
    -webkit-mask: url('/owl.png') center / contain no-repeat;
}

.app-logo-img {
    display: inline-block;
    height: 1em;
    width: auto;
    max-width: 8em;
    object-fit: contain;
    vertical-align: middle;
}
</style>
