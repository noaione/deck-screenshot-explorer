<template>
  <RouterView />
</template>

<script setup lang="ts">
const router = useRouter();

onMounted(() => {
  // check if has redirect query
  const windowSearch = new URLSearchParams(window.location.search);
  const redirect = windowSearch.get("redirect");

  if (redirect) {
    console.info(`Redirecting to: ${redirect}`);
    router.push(decodeURIComponent(redirect));
  }
});
</script>
