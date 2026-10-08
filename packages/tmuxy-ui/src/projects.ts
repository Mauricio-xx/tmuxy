import { useEffect, useState } from 'react';
import { isTauri } from './tmux/adapters';

/** A project from `~/.config/tmuxy/projects.json`. */
export interface SessionProject {
  name: string;
  sessions: string[];
}

export interface GroupedSession {
  name: string;
  /** `null` when the session is not in any project. */
  project: string | null;
}

const STASH_SESSION = '__tmuxy_stash';

/**
 * Order live session names the way the switcher draws them: each project's
 * sessions in file order, then the sessions that belong to nobody, in the
 * order tmux listed them. A session named in two projects stays with the
 * first. The stash session is not a session the user switches to.
 */
export function groupSessions(names: string[], projects: SessionProject[]): GroupedSession[] {
  const live = names.filter((name) => name.length > 0 && name !== STASH_SESSION);
  const seen = new Set<string>();
  const grouped: GroupedSession[] = [];

  for (const project of projects) {
    for (const session of project.sessions) {
      if (!live.includes(session) || seen.has(session)) continue;
      seen.add(session);
      grouped.push({ name: session, project: project.name });
    }
  }
  for (const name of live) {
    if (seen.has(name)) continue;
    grouped.push({ name, project: null });
  }
  return grouped;
}

/** Read projects once. Empty on the web build, which has no Tauri command. */
export function useSessionProjects(): SessionProject[] {
  const [projects, setProjects] = useState<SessionProject[]>([]);
  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    void import('@tauri-apps/api/core').then(({ invoke }) =>
      invoke<SessionProject[]>('list_projects')
        .then((rows) => {
          if (!cancelled && Array.isArray(rows)) setProjects(rows);
        })
        .catch(() => {}),
    );
    return () => {
      cancelled = true;
    };
  }, []);
  return projects;
}
