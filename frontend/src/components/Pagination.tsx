import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { ChevronLeft, ChevronRight } from 'lucide-react'

interface PaginationProps {
  total: number
  page: number
  pageSize: number
  onPageChange: (page: number) => void
  onPageSizeChange?: (size: number) => void
  pageSizeOptions?: number[]
}

export function Pagination({
  total,
  page,
  pageSize,
  onPageChange,
  onPageSizeChange,
  pageSizeOptions,
}: PaginationProps) {
  const totalPages = Math.max(1, Math.ceil(total / pageSize))
  const [jumpPage, setJumpPage] = useState('')

  if (total <= 0) return null

  return (
    <div className="flex items-center justify-between px-4 py-2.5 border-t">
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <span>共 {total} 条</span>
        {onPageSizeChange && pageSizeOptions && (
          <select
            value={pageSize}
            onChange={(e) => onPageSizeChange(Number(e.target.value))}
            className="h-7 rounded-md border border-input bg-background px-2 text-xs focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary/30"
          >
            {pageSizeOptions.map(s => (
              <option key={s} value={s}>{s} 条/页</option>
            ))}
          </select>
        )}
      </div>
      <div className="flex items-center gap-1.5">
        <Button variant="outline" size="icon" className="h-7 w-7" disabled={page <= 1} onClick={() => onPageChange(page - 1)}>
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
        <span className="px-2 text-xs tabular-nums text-muted-foreground">{page} / {totalPages}</span>
        <Button variant="outline" size="icon" className="h-7 w-7" disabled={page >= totalPages} onClick={() => onPageChange(page + 1)}>
          <ChevronRight className="h-3.5 w-3.5" />
        </Button>
        <input
          type="number"
          min={1}
          max={totalPages}
          value={jumpPage}
          onChange={(e) => setJumpPage(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              const p = Math.max(1, Math.min(totalPages, Number(jumpPage) || 1))
              onPageChange(p)
              setJumpPage('')
            }
          }}
          placeholder="跳转"
          className="h-7 w-14 rounded-md border border-input bg-background px-2 text-xs text-center focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary/30 ml-1"
        />
      </div>
    </div>
  )
}
