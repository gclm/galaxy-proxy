import { ToggleSwitch } from '@/components/ToggleSwitch'

export function StatusBadge({ enabled, onClick }: { enabled: boolean; onClick: () => void }) {
  return <ToggleSwitch enabled={enabled} onClick={onClick} size="sm" />
}
