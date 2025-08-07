import "./globals.css";
import { AuthProvider } from "@/context/AuthContext";
import Navbar from "@/components/Navbar";

export const metadata = {
    title: "CTF Dashboard",
};

export default function RootLayout({
                                       children,
                                   }: {
    children: React.ReactNode;
}) {
    return (
        <html lang="en">
        <body className="min-h-screen bg-gray-50">
        <AuthProvider>
            <Navbar/>
            <main className="p-6">{children}</main>
        </AuthProvider>
        </body>
        </html>
    );
}
