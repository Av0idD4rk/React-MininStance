"use client";

import { createContext, useState, useEffect, ReactNode } from "react";

interface AuthContextType {
    token: string | null;
    initialized: boolean;
    login: (username: string) => Promise<void>;
    logout: () => void;
}

export const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
    const [token, setToken] = useState<string | null>(null);
    const [initialized, setInitialized] = useState(false);  // NEW


    useEffect(() => {
        setToken(localStorage.getItem("ctf_token"));
        setInitialized(true);
    }, []);

    async function login(username: string) {
        const res = await fetch("http://ctf.av0idd4rk.ru:8080/token", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ username }),
        });
        if (!res.ok) throw new Error("Login failed");
        const { token: t } = await res.json();
        localStorage.setItem("ctf_token", t);
        setToken(t);
    }

    function logout() {
        localStorage.removeItem("ctf_token");
        setToken(null);
    }

    return (
        <AuthContext.Provider value={{ token, initialized, login, logout }}>
            {children}
        </AuthContext.Provider>
    );
}
