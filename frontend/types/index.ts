export interface TaskEntry {
    name: string;
    protocol: "http" | "tcp";
    container_port: number;
}

export interface Instance {
    id: number;
    task_name: string;
    expires_in_secs: number;
    container_id: string;
    endpoint: string;
    status: "Running" | "Stopped" | "Expired";
}
