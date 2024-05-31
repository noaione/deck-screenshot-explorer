<template>
  <RouterLink
    v-if="userId !== undefined"
    :to="`/s/${userId}/${app.id}`"
    class="flex cursor-pointer items-center rounded-md bg-gray-300 shadow-md transition-colors hover:bg-gray-200 dark:bg-gray-800 hover:dark:bg-gray-700"
  >
    <div class="flex w-full items-center justify-between px-4 py-4">
      <div class="flex w-full items-center">
        <i-mdi-steam v-if="!app.non_steam" class="mr-2 h-4 w-4" />
        <i-mdi-application v-else class="mr-2 h-4 w-4" />
        <div class="pointer-events-none text-lg font-semibold">{{ app.name }}</div>
      </div>
    </div>
  </RouterLink>
  <div v-else class="flex w-full flex-col px-4 py-4">
    <div class="text-lg font-semibold">{{ app.name }}</div>
    <div class="mt-2 text-sm font-light opacity-80">{{ app.id }}</div>
    <span v-if="app.non_steam" class="mt-1 text-sm font-light opacity-80">Non-Steam Apps</span>
    <div v-if="app.developers.length > 0" class="mt-2">
      <div class="text-sm font-semibold">Developers</div>
      <div class="flex flex-wrap gap-2">
        <p v-for="dev in app.developers" :key="dev">{{ dev }}</p>
      </div>
    </div>
    <div v-if="app.publishers.length > 0" class="mt-2">
      <div class="text-sm font-semibold">Publishers</div>
      <div class="flex flex-wrap">
        <p v-for="(dev, idx) in app.publishers" :key="dev" :class="`${idx !== 0 ? 'ml-1' : ''}`">
          {{ `${dev}${idx !== app.publishers.length - 1 ? ", " : " "}` }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { AppInfo } from "@/composables/use-backend-fetch";

defineProps<{
  userId?: number;
  app: AppInfo;
}>();
</script>
