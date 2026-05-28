export function StatusBadge({ enabled, onClick }: { enabled: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium transition-colors cursor-pointer ${
        enabled
          ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
          : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400'
      }`}
    >
      {enabled ? '启用' : '禁用'}
    </button>
  )
}
