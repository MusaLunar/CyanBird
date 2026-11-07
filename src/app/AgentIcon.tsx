import type { ProviderId } from '../types/domain';
export function AgentIcon({ provider }: { provider: ProviderId }) {
  return <span className={`provider-mark ${provider}`}><img src={`/agents/${provider}.svg`} alt="" aria-hidden="true" /></span>;
}
