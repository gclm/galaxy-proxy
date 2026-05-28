import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

export function ConfirmDeleteDialog({
  open,
  onOpenChange,
  title = '确认删除',
  message = '此操作不可撤销。',
  onConfirm,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title?: string
  message?: string
  onConfirm: () => void
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">{message}</p>
        <div className="flex justify-end gap-2 mt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button variant="destructive" onClick={onConfirm}>删除</Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
