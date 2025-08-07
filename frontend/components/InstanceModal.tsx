"use client";

import {FC, useState} from "react";
import {useEffect} from "react";
import {Dialog, DialogTrigger, DialogContent, DialogHeader, DialogFooter, DialogTitle} from "@/components/ui/dialog";
import {Button} from "@/components/ui/button";
import {API_URL, authFetch} from "@/lib/api";
import {Instance} from "@/types";
import {ClockIcon} from "@heroicons/react/24/outline";

export interface InstanceModalProps {
    taskName: string;
    token: string;
    onClose: () => void;
    onDeploy: (inst: Instance, endpoint: string) => void;
    onAction: (
        action: "stop" | "restart" | "extend",
        instanceId: number
    ) => Promise<void>;
}

const InstanceModal: FC<InstanceModalProps> = ({
                                                   taskName, token, onDeploy, onClose, onAction
                                               }) => {
    const [busy, setBusy] = useState(false);
    const [inst, setInst] = useState<Instance | null>(null);
    const [endpoint, setEndpoint] = useState<string>("");

    const [secondsLeft, setSecondsLeft] = useState<number>(0);

    async function start() {
        setBusy(true);
        const res = await authFetch(`${API_URL}/deploy`, token, {
            method: "POST",
            body: JSON.stringify({
                captcha_token: "null",
                task: taskName,
            }),
        });
        const data: { instance: Instance } = await res.json();
        setSecondsLeft(data.instance.expires_in_secs);
        setInst(data.instance);
        setEndpoint(data.instance.endpoint);
        onDeploy(data.instance, data.instance.endpoint);
        setBusy(false);
    }

    // tick the countdown every second
    useEffect(() => {
        if (secondsLeft <= 0) return;
        const id = setInterval(() => {
            setSecondsLeft((s) => Math.max(0, s - 1));
        }, 1000);
        return () => clearInterval(id);
    }, [secondsLeft]);

    async function act(
        action: "stop" | "restart" | "extend"
    ) {
        if (!inst) return;
        setBusy(true);
        await onAction(action, inst.id);
        setBusy(false);
    }

    return (
        <Dialog open onOpenChange={onClose}>
            <DialogTrigger asChild></DialogTrigger>
            <DialogContent>
                <DialogTitle>Instance</DialogTitle>
                <DialogHeader>
                    <h3 className="text-lg font-medium">{taskName}</h3>
                </DialogHeader>

                {!inst ? (
                    <div className="space-y-4">
                        <Button onClick={start} disabled={busy}>
                            {busy ? "Starting…" : "Start Instance"}
                        </Button>
                    </div>
                ) : (
                    <div className="space-y-4">
                        {/* Endpoint display */}
                        <div>
                            <label className="font-semibold">Endpoint:</label>
                            <a href={endpoint} className="mt-1 text-blue-600 underline break-all">
                                {endpoint}
                            </a>
                        </div>

                        <div className="flex items-center space-x-2 text-gray-700">
                            <ClockIcon className="h-5 w-5"/>
                            <span>
                Expires in{" "}
                                {Math.floor(secondsLeft / 60)}m {secondsLeft % 60}s
              </span>
                        </div>
                        <div className="flex gap-2">
                            <Button onClick={() => act("extend")} disabled={busy}>
                                Extend
                            </Button>
                            <Button onClick={() => act("restart")} disabled={busy}>
                                Restart
                            </Button>
                            <Button variant="destructive" onClick={() => act("stop")} disabled={busy}>
                                Stop
                            </Button>
                        </div>
                    </div>
                )}

                <DialogFooter>
                    <Button variant="secondary" onClick={onClose}>
                        Close
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default InstanceModal;
