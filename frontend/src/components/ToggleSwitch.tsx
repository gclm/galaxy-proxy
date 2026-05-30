interface ToggleSwitchProps {
  enabled: boolean
  onClick: () => void
  size?: 'sm' | 'md'
  disabled?: boolean
}

export function ToggleSwitch({ enabled, onClick, size = 'sm', disabled }: ToggleSwitchProps) {
  const sizes = size === 'sm'
    ? { track: 'h-5 w-9', thumb: 'h-4 w-4', on: 'translate-x-4', off: 'translate-x-0.5' }
    : { track: 'h-6 w-11', thumb: 'h-5 w-5', on: 'translate-x-5', off: 'translate-x-0.5' }

  return (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      disabled={disabled}
      onClick={onClick}
      title={enabled ? '点击禁用' : '点击启用'}
      className={`relative inline-flex shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/30 disabled:cursor-not-allowed disabled:opacity-50 ${sizes.track} ${
        enabled ? 'bg-primary' : 'bg-muted'
      }`}
    >
      <span
        className={`pointer-events-none inline-block rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${sizes.thumb} ${enabled ? sizes.on : sizes.off}`}
      />
    </button>
  )
}
