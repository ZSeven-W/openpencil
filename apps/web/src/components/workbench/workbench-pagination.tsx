import {
  ChevronLeftIcon,
  ChevronRightIcon,
} from 'lucide-react';
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
} from '@/components/ui/pagination';

interface WorkbenchPaginationProps {
  page: number;
  pageSize: number;
  total: number;
  onPageChange: (page: number) => void;
  labels: {
    range: (values: { from: number; to: number; total: number }) => string;
    previous: string;
    next: string;
  };
}

function paginationPages(page: number, pageCount: number): number[] {
  if (pageCount <= 5) return Array.from({ length: pageCount }, (_, index) => index + 1);
  const start = Math.max(1, Math.min(page - 2, pageCount - 4));
  return Array.from({ length: 5 }, (_, index) => start + index);
}

export function WorkbenchPagination({
  page,
  pageSize,
  total,
  onPageChange,
  labels,
}: WorkbenchPaginationProps) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const currentPage = Math.min(Math.max(1, page), pageCount);
  const from = total === 0 ? 0 : (currentPage - 1) * pageSize + 1;
  const to = Math.min(total, currentPage * pageSize);
  const pages = paginationPages(currentPage, pageCount);

  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <p className="text-xs text-muted-foreground">{labels.range({ from, to, total })}</p>
      <Pagination className="mx-0 w-auto text-xs">
        <PaginationContent>
          <PaginationItem>
            <PaginationLink
              aria-disabled={currentPage <= 1}
              aria-label={labels.previous}
              className={currentPage <= 1 ? 'pointer-events-none opacity-50' : undefined}
              href="#"
              size="default"
              onClick={(event) => {
                event.preventDefault();
                if (currentPage > 1) onPageChange(currentPage - 1);
              }}
            >
              <ChevronLeftIcon />
              {labels.previous}
            </PaginationLink>
          </PaginationItem>
          {pages.map((item) => (
            <PaginationItem key={item}>
              <PaginationLink
                href="#"
                isActive={item === currentPage}
                onClick={(event) => {
                  event.preventDefault();
                  if (item !== currentPage) onPageChange(item);
                }}
              >
                {item}
              </PaginationLink>
            </PaginationItem>
          ))}
          <PaginationItem>
            <PaginationLink
              aria-disabled={currentPage >= pageCount}
              aria-label={labels.next}
              className={currentPage >= pageCount ? 'pointer-events-none opacity-50' : undefined}
              href="#"
              size="default"
              onClick={(event) => {
                event.preventDefault();
                if (currentPage < pageCount) onPageChange(currentPage + 1);
              }}
            >
              {labels.next}
              <ChevronRightIcon />
            </PaginationLink>
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  );
}
