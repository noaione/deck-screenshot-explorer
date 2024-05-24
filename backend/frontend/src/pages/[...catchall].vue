<template>
  <main class="mx-auto my-auto flex h-screen w-full flex-col items-center justify-center">
    <i-mdi-web-off class="mb-2 h-12 w-12" />
    <div class="font-variable variation-weight-bold mb-2 text-xl">Page Not found</div>
    <code
      class="break-all rounded-sm bg-red-900 bg-opacity-75 px-2 py-1 font-mono text-white opacity-80 dark:bg-opacity-25"
    >
      {{ $route.fullPath }}
    </code>
  </main>
  <footer-info />
</template>

<script setup lang="ts">
useHeadSafe({
  title: "Page Not Found - 404 :: Deck Screenshot Explorer",
});
</script>
