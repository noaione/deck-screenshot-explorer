<template>
  <div class="flex flex-col bg-gray-200 dark:bg-gray-800">
    <div class="flex w-full flex-col object-contain">
      <img :src="buildThumbUrl(userId, appId, filename)" class="h-32 w-auto" />
    </div>
    <div class="flex flex-col justify-center px-2 py-1 text-center text-sm">
      <button class="hover:underline hover:opacity-80" @click="downloadScreenshot(userId, appId, filename)">
        {{ filename }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  userId: string;
  appId: string;
  filename: string;
}>();

function buildThumbUrl(userId: string, appId: string, filename: string) {
  return makeUrl(`/users/${userId}/${appId}/t/${filename}`);
}

function buildMainUrl(userId: string, appId: string, filename: string) {
  return makeUrl(`/users/${userId}/${appId}/${filename}`);
}

function downloadScreenshot(userId: string, appId: string, filename: string) {
  const url = buildMainUrl(userId, appId, filename);

  const a = document.createElement("a");

  a.href = url;
  a.download = filename;
  // hide
  a.style.display = "none";

  // append
  document.body.append(a);

  // trigger then remove
  a.click();
  a.remove();
}
</script>
