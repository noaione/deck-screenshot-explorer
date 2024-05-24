<template>
  <RouterLink
    :to="`/s/${user.id3}`"
    class="flex w-full cursor-pointer items-center rounded-md bg-gray-300 shadow-md transition-colors hover:bg-gray-200 dark:bg-gray-800 hover:dark:bg-gray-700"
  >
    <div class="flex w-full items-center justify-between px-4 py-4">
      <div class="flex w-full items-center">
        <i-mdi-account-circle class="mr-2 h-6 w-6" />
        <div class="pointer-events-none text-xl font-bold">{{ user.displayName || user.username }}</div>
      </div>
    </div>
  </RouterLink>
</template>

<script setup lang="ts">
import type { User } from "@/composables/use-backend-fetch";

defineProps<{
  user: User;
}>();
</script>
