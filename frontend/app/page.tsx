'use client';

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";
import { fetcher, authFetch, API_URL } from "@/lib/api";
import { TaskEntry, Instance } from "@/types";
import TaskCard from "@/components/TaskCard";
import InstanceCard from "@/components/InstanceCard";
import InstanceModal from "@/components/InstanceModal";

export default function HomePage() {
    const { token,initialized } = useAuth();
    const router = useRouter();

    const [instances, setInstances] = useState<Instance[]>([]);
    const [selected, setSelected] = useState<string | null>(null);
    const [tasks, setTasks] = useState<TaskEntry[]>([]);

    useEffect(() => {
        if (!token) {
            router.push("/login");
        }

        fetcher<TaskEntry[]>(`${API_URL}/tasks`)
            .then((arr) => {
                console.log("Loaded tasks:", arr);
                setTasks(arr);
            })
            .catch(console.error);

        reloadInstances();
    }, [initialized, token, router]);

    async function reloadInstances() {
        const list = await authFetch(`${API_URL}/instances`, token!).then(r => r.json());
        setInstances(list);
        console.log(list)
    }
    function handleDeploy(inst: Instance, endpoint: string) {
        reloadInstances();
    }
    async function handleAction(action: "stop"|"restart"|"extend", id: number) {
        await authFetch(`${API_URL}/${action}`, token!, {
            method: "POST",
            body: JSON.stringify({ instance_id: id }),
        });
        reloadInstances();
    }

    return (
        <>
            {/* Render the task cards from the *tasks* array */}
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6">
                {tasks.map((task) => (
                    <TaskCard
                        key={task.name}
                        name={task.name}
                        protocol={task.protocol}
                        onClick={() => setSelected(task.name)}
                    />
                ))}
            </div>

            <section className="mt-12 space-y-4">
                {instances.map((inst) => (
                    <InstanceCard
                        key={inst.id}
                        id={inst.id}
                        task_name={inst.task_name}
                        expires_in_secs={inst.expires_in_secs}
                        endpoint={inst.endpoint}
                        onAction={handleAction}
                    />
                ))}
            </section>

            {selected && (
                <InstanceModal
                    taskName={selected}
                    token={token!}
                    onClose={() => setSelected(null)}
                    onAction={handleAction}
                    onDeploy={handleDeploy}
                />
            )}
        </>
    );
}
