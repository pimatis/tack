import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '$lib/db/client';
import { getSettings } from '$lib/stores/settings';

export type AgentStatus = { port: number; url: string };

// keep the embedded agent server in sync with the agentEnabled/agentPort
// settings, including changes made from the cli (tack settings set). unlike
// live this one persists across launches: ai agents need it always on
export function startAgentManager(): () => void {
	if (!isTauri()) return () => {};
	const reconcile = () => void reconcileAgent();
	window.addEventListener('settings-changed', reconcile);
	void reconcileAgent();
	return () => window.removeEventListener('settings-changed', reconcile);
}

let reconciling = false;
let retried = false;

async function reconcileAgent(): Promise<void> {
	if (reconciling) return;
	reconciling = true;
	try {
		const { agentEnabled, agentPort } = getSettings();
		const status = await getAgentStatus();
		if (agentEnabled && status && status.port === agentPort) return;
		if (!agentEnabled && !status) return;
		if (agentEnabled) {
			if (status) await invoke('agent_stop');
			const next = await invoke<AgentStatus>('agent_start', { port: agentPort });
			dispatchAgentStatus(next);
		} else {
			await invoke('agent_stop');
			dispatchAgentStatus(null);
		}
		retried = false;
	} catch (error) {
		// start failed (usually a transient bind race during restarts): keep
		// the setting on and retry once shortly, only surface the error
		window.dispatchEvent(new CustomEvent('agent-error-changed', { detail: String(error) }));
		if (!retried && getSettings().agentEnabled) {
			retried = true;
			reconciling = false;
			await new Promise((resolve) => setTimeout(resolve, 2000));
			void reconcileAgent();
			return;
		}
	} finally {
		reconciling = false;
	}
}

export async function getAgentStatus(): Promise<AgentStatus | null> {
	try {
		return await invoke<AgentStatus | null>('agent_status');
	} catch {
		return null;
	}
}

function dispatchAgentStatus(status: AgentStatus | null): void {
	window.dispatchEvent(new CustomEvent('agent-status-changed', { detail: status }));
}
