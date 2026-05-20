import { get } from 'svelte/store';
import { activeTabId, tabs } from '$lib/shared/stores/tabs';
import { getSqlTabData, connections as sqlConnections, databaseTables, connectedIds } from '../stores';
import type { ChatContext, ContextRequest, ContextResponse, ContextEnvVar } from '$lib/types/ai';

export async function gatherSqlContext(): Promise<ChatContext> {
  const tabId = get(activeTabId);
  const tabData = getSqlTabData(tabId);
  const conns = get(sqlConnections);
  const allConnected = get(connectedIds);

  const sqlTab = get(tabs).find((t) => t.id === tabId && t.mode === 'sql');
  const hasSqlTab = !!sqlTab;
  const tabConnId = tabData.binding?.connectionId || null;
  const tabDb = tabData.binding?.database || null;
  const targetConn = tabConnId ? conns.find((c: any) => c.id === tabConnId) : undefined;

  let targetStatus: 'ready' | 'database_unselected' | 'no_binding' | 'no_sql_tab';
  if (!hasSqlTab) {
    targetStatus = 'no_sql_tab';
  } else if (!targetConn) {
    targetStatus = 'no_binding';
  } else if (!tabDb) {
    targetStatus = 'database_unselected';
  } else {
    targetStatus = 'ready';
  }

  let currentRequest: ContextRequest | null = null;
  if (tabData.query) {
    currentRequest = {
      method: 'SQL', url: '', headers: [], params: [],
      body: tabData.query,
      bodyType: 'sql',
      authType: 'none', authData: '{}',
    };
  }

  let currentResponse: ContextResponse | null = null;
  if (tabData.results && tabData.results.length > 0) {
    const activeResult = tabData.results[tabData.activeResultIdx || 0];
    if (activeResult?.result) {
      const r = activeResult.result;
      const rowCount = r.rows?.length || r.affectedRows || 0;
      const preview = `${rowCount} rows. Columns: ${(r.columns || []).join(', ')}`;
      currentResponse = {
        status: 200, statusText: `${rowCount} rows`,
        headers: (r.columns || []).map((c: string) => [c, 'column'] as [string, string]),
        body: preview,
        durationMs: r.durationMs || 0, sizeBytes: 0,
      };
    } else if (activeResult?.error) {
      currentResponse = {
        status: 500, statusText: 'Error',
        headers: [], body: activeResult.error,
        durationMs: 0, sizeBytes: 0,
      };
    }
  }

  const envVars: ContextEnvVar[] = [];
  envVars.push({ key: 'target_status', value: targetStatus, isSecret: false });

  if (targetStatus === 'ready' && targetConn && tabDb) {
    envVars.push({ key: 'connection_id', value: targetConn.id, isSecret: false });
    envVars.push({ key: 'saved_connection_id', value: targetConn.id, isSecret: false });
    envVars.push({ key: 'connection_name', value: targetConn.name, isSecret: false });
    envVars.push({ key: 'driver', value: targetConn.driver, isSecret: false });
    envVars.push({ key: 'database', value: tabDb, isSecret: false });
    envVars.push({ key: 'pool_status', value: allConnected.has(targetConn.id) ? 'live' : 'idle', isSecret: false });

    const tables = get(databaseTables);
    const tableList = tables.get(`${targetConn.id}:${tabDb}`);
    if (tableList && tableList.length > 0) {
      const schema = tableList.slice(0, 30).map((t: any) => {
        const cols = (t.columns || []).map((c: any) => c.name).join(', ');
        return `${t.name}(${cols})`;
      }).join('\n');
      envVars.push({ key: 'schema', value: schema, isSecret: false });
    }
  } else if (targetStatus === 'database_unselected' && targetConn) {
    envVars.push({ key: 'partial_connection_id', value: targetConn.id, isSecret: false });
    envVars.push({ key: 'partial_connection_name', value: targetConn.name, isSecret: false });
    envVars.push({ key: 'partial_driver', value: targetConn.driver, isSecret: false });
  }

  const available = conns
    .filter((c: any) => allConnected.has(c.id))
    .map((c: any) => `${c.name} (saved_id=${c.id}, driver=${c.driver}, default_db=${c.databaseName})`);
  if (available.length > 0) {
    envVars.push({ key: 'available_connections', value: available.join('\n'), isSecret: false });
  } else {
    envVars.push({ key: 'available_connections', value: 'none — user has no live SQL connections', isSecret: false });
  }

  return { mode: 'sql', currentRequest, currentResponse, envVars };
}
