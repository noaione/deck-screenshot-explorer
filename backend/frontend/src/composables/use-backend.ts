import useBackendFetch from "./use-backend-fetch";

interface OptionsBackend {
  immediate?: boolean;
}

export default function useBackend<T>(url: string, fetchOptions?: RequestInit, options?: OptionsBackend) {
  const mergedOptions: OptionsBackend = {
    immediate: true,
    ...options,
  };

  const data = ref<T>();
  const error = ref();

  const loading = ref(false);

  async function fetchData(): Promise<T> {
    loading.value = true;

    try {
      const resp = await useBackendFetch<T>(url, fetchOptions);

      loading.value = false;
      data.value = resp;

      return resp;
    } catch (_error) {
      console.error(_error);

      loading.value = false;
      error.value = _error;

      throw _error;
    }
  }

  async function reload(): Promise<T> {
    data.value = undefined;
    error.value = undefined;

    return await fetchData();
  }

  if (mergedOptions.immediate) {
    fetchData();
  }

  return {
    data,
    error,
    loading,
    fetch: fetchData,
    reload,
  };
}
