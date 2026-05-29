export function StatusBadge({ enabled, onClick }: { enabled: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      title={enabled ? '点击禁用' : '点击启用'}
      className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
        enabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'
      }`}
    >
      <span
        className={`inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform ${
          enabled ? 'translate-x-4' : 'translate-x-0.5'
        }`}
      />
    </button>
  )
}
