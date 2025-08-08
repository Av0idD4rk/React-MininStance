"use client";

import { FC } from "react";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/hooks/useAuth";

const Navbar: FC = () => {
    const { token, logout } = useAuth();
    return (
        <nav className="bg-white border-b px-6 py-3 flex justify-between items-center">
            <h1 className="text-2xl font-bold">CTF Dashboard</h1>
            {token && (
                <Button size="sm" variant="ghost" onClick={logout}>
                    Logout
                </Button>
            )}
        </nav>
    );
};

export default Navbar;
