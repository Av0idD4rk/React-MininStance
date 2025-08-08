export const API_URL = "http://ctf.av0idd4rk.ru:8080";

export async function fetcher<T>(input: RequestInfo, init?: RequestInit): Promise<T> {
    const res = await fetch(input, init);
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return (await res.json()) as T;
}

export function authFetch(input: RequestInfo, token: string, init?: RequestInit) {
    return fetch(input, {
        ...init,
        headers: {
            ...(init?.headers as any),
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
        },
    });
}
