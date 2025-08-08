import { FC } from "react";
import { Card, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ServerIcon, ShieldCheckIcon } from "@heroicons/react/24/outline";

interface TaskCardProps {
    name: string;
    protocol: "http" | "tcp";
    onClick: () => void;
}

const icons = {
    http: ServerIcon,
    tcp: ShieldCheckIcon,
};

const TaskCard: FC<TaskCardProps> = ({ name, protocol, onClick }) => {
    const Icon = icons[protocol];
    return (
        <Card className="group hover:shadow-lg transition-shadow">
            <CardHeader>
                <div className="flex items-center space-x-2">
                    <Icon className="h-6 w-6 text-indigo-500 group-hover:animate-pulse" />
                    <CardTitle className="capitalize">{name.replace(/_/g, " ")}</CardTitle>
                </div>
                <CardDescription className="capitalize text-gray-500">
                    {protocol}
                </CardDescription>
            </CardHeader>
            <div className="p-4">
                <Button onClick={onClick} className="w-full">
                    View
                </Button>
            </div>
        </Card>
    );
};

export default TaskCard;
