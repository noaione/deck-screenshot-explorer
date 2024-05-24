<template>
  <div class="flex w-full flex-col items-center justify-center">
    <div class="flex items-center justify-between gap-2 px-4 py-4">
      <button
        @click="() => $emit('update', page - 1)"
        class="flex h-6 w-6 items-center justify-center rounded-md bg-gray-300 transition hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-70 disabled:hover:opacity-70 dark:bg-gray-700"
        :disabled="page === 0"
      >
        <i-mdi-chevron-left class="h-6 w-6" />
      </button>
      <div class="flex w-full items-center">
        <div class="pointer-events-none text-lg">{{ page + 1 }} of {{ actualPage }}</div>
      </div>
      <button
        @click="() => $emit('update', page + 1)"
        class="flex h-6 w-6 items-center justify-center rounded-md bg-gray-300 transition hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-70 disabled:hover:opacity-70 dark:bg-gray-700"
        :disabled="page + 1 === actualPage"
      >
        <i-mdi-chevron-right class="h-6 w-6" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  page: number;
  total: number;
}>();

defineEmits<{
  (e: "update", page: number): void;
}>();

const deck = useDeckStore();

const actualPage = computed(() => {
  // upper bound
  return Math.floor(props.total / deck.per_page) + 1;
});
</script>
