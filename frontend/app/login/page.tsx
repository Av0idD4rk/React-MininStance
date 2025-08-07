"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export default function LoginPage() {
    const { login } = useAuth();
    const [user, setUser] = useState("");
    const [err, setErr] = useState<string | null>(null);
    const router = useRouter();

    async function onSubmit(e: React.FormEvent) {
        e.preventDefault();
        try {
            await login(user);
            router.push("/");
        } catch (e: any) {
            setErr(e.message);
        }
    }

    return (
        <div className="max-w-md mx-auto mt-20 bg-white p-8 rounded shadow">
            <h2 className="text-xl font-semibold mb-4">Sign In</h2>
            <form onSubmit={onSubmit} className="space-y-4">
                <Input
                    placeholder="Username"
                    value={user}
                    onChange={(e) => setUser(e.target.value)}
                />
                {err && <p className="text-red-600">{err}</p>}
                <Button type="submit">Get Token</Button>
            </form>
        </div>
    );
}
