// components/InstanceCard.tsx
"use client";

import React, { FC, useEffect, useState } from "react";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
    ClockIcon,
    ArrowPathIcon,
    XCircleIcon,
    PlusCircleIcon,
} from "@heroicons/react/24/outline";

export interface InstanceCardProps {
    id: number;
    task_name: string;
    expires_in_secs: number;      // new
    endpoint: string;
    onAction: (action: "stop" | "restart" | "extend", id: number) => void;
}

const InstanceCard: FC<InstanceCardProps> = ({
                                                 id,
                                                 task_name,
                                                 expires_in_secs,
                                                 endpoint,
                                                 onAction,
                                             }) => {
    // initialize with the server-given TTL
    const [secsLeft, setSecsLeft] = useState(expires_in_secs);

    // count down once per second
    useEffect(() => {
        if (secsLeft <= 0) return;
        const iv = setInterval(() => {
            setSecsLeft((s) => Math.max(0, s - 1));
        }, 1000);
        return () => clearInterval(iv);
    }, [secsLeft]);

    // format minutes/seconds
    const minutes = Math.floor(secsLeft / 60);
    const seconds = secsLeft % 60;

    // progress bar percentage
    const pct =
        expires_in_secs > 0
            ? Math.round((secsLeft / expires_in_secs) * 100)
            : 0;

    return (
        <Card className="border-l-4 border-indigo-500">
            <CardContent className="space-y-2">
                <div className="flex justify-between items-center">
          <span className="font-semibold capitalize">
            {task_name.replace(/_/g, " ")}
          </span>
                    <span className="flex items-center text-sm text-gray-600">
            <ClockIcon className="h-4 w-4 mr-1" />
                        {minutes}m {seconds}s
          </span>
                </div>

                {/* display the real endpoint from server */}
                <pre className="bg-gray-100 p-2 rounded overflow-x-auto text-sm break-all">
          {endpoint}
        </pre>

                {/* progress bar */}
                <div className="w-full bg-gray-200 rounded-full h-2">
                    <div
                        className="bg-indigo-500 h-2 rounded-full transition-all"
                        style={{ width: `${pct}%` }}
                    />
                </div>
            </CardContent>

            <CardFooter className="flex justify-end space-x-2">
                <Button size="sm" onClick={() => onAction("extend", id)}>
                    <PlusCircleIcon className="h-5 w-5 mr-1" /> Extend
                </Button>
                <Button size="sm" onClick={() => onAction("restart", id)}>
                    <ArrowPathIcon className="h-5 w-5 mr-1" /> Restart
                </Button>
                <Button
                    size="sm"
                    variant="destructive"
                    onClick={() => onAction("stop", id)}
                >
                    <XCircleIcon className="h-5 w-5 mr-1" /> Stop
                </Button>
            </CardFooter>
        </Card>
    );
};

export default InstanceCard;
