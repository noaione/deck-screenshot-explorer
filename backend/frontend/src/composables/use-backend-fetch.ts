export interface User {
  id: number;
  id3: number;
  id64: number;
  username: string;
  displayName: string;
  timestamp: number;
}

export interface AppInfo {
  id: number;
  name: string;
  localized_name: { [key: string]: string };
  developers: string[];
  publishers: string[];
  non_steam: boolean;
}

export interface Pagination {
  total: number;
  page: number;
  per_page: number;
}

export interface AppInfoWithScreenshots {
  app: AppInfo;
  screenshots: string[];
  pagination: Pagination;
}

export function makeUrl(url: string): string {
  const baseHost = import.meta.env.VITE_BASE_HOST;

  if (url.startsWith("/")) {
    url = url.slice(1);
  }

  if (!url.startsWith("api/")) {
    url = `api/${url}`;
  }

  if (baseHost) {
    if (baseHost.endsWith("/")) {
      return `${baseHost}${url}`;
    }

    return `${baseHost}/${url}`;
  }

  return `/${url}`;
}

export default function useBackendFetch<T>(url: string, fetchOptions?: RequestInit): Promise<T> {
  const mergedFetchOptions: RequestInit = {
    ...fetchOptions,
  };

  return new Promise<T>((resolve, reject) => {
    fetch(makeUrl(url), mergedFetchOptions)
      .then((resp) => {
        if (resp.ok) {
          return resp.json();
        }

        throw new Error(resp.statusText);
      })
      .then((json) => {
        if (json.ok) {
          resolve(json.data);
        } else {
          reject(json.error);
        }
      })
      .catch((error) => {
        reject(error);
      });
  });
}
