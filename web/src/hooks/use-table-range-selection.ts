import type { KeyboardEvent, PointerEvent } from "react";
import { useCallback, useEffect, useMemo, useRef } from "react";

type CheckedState = boolean | "indeterminate";

interface UseTableRangeSelectionArgs {
  itemIds: string[];
  selectedIds: string[];
  onSelectedIdsChange: (selectedIds: string[]) => void;
  disabledIds?: string[];
}

interface DragState {
  active: boolean;
  targetChecked: boolean;
}

function mergeSelection(current: string[], targetIds: string[], checked: boolean) {
  const next = new Set(current);
  for (const targetId of targetIds) {
    if (checked) {
      next.add(targetId);
    } else {
      next.delete(targetId);
    }
  }
  return [...next];
}

export function useTableRangeSelection({
  itemIds,
  selectedIds,
  onSelectedIdsChange,
  disabledIds = [],
}: UseTableRangeSelectionArgs) {
  const selectedRef = useRef(selectedIds);
  const anchorIdRef = useRef<string | null>(null);
  const dragStateRef = useRef<DragState>({ active: false, targetChecked: false });
  const suppressNextCheckedChangeRef = useRef(false);

  useEffect(() => {
    selectedRef.current = selectedIds;
  }, [selectedIds]);

  useEffect(() => {
    const stopDrag = () => {
      dragStateRef.current = { active: false, targetChecked: false };
    };

    window.addEventListener("pointerup", stopDrag);
    window.addEventListener("pointercancel", stopDrag);
    return () => {
      window.removeEventListener("pointerup", stopDrag);
      window.removeEventListener("pointercancel", stopDrag);
    };
  }, []);

  const disabledSet = useMemo(() => new Set(disabledIds), [disabledIds]);
  const enabledItemIds = useMemo(
    () => itemIds.filter((itemId) => !disabledSet.has(itemId)),
    [disabledSet, itemIds],
  );
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const selectedEnabledCount = enabledItemIds.filter((itemId) => selectedSet.has(itemId)).length;
  const allSelected = enabledItemIds.length > 0 && selectedEnabledCount === enabledItemIds.length;
  const someSelected = selectedEnabledCount > 0;
  const selectAllChecked: CheckedState = allSelected
    ? true
    : someSelected
      ? "indeterminate"
      : false;

  const commitSelection = useCallback(
    (targetIds: string[], checked: boolean) => {
      const eligibleTargetIds = targetIds.filter((targetId) => !disabledSet.has(targetId));
      if (eligibleTargetIds.length === 0) {
        return;
      }

      const next = mergeSelection(selectedRef.current, eligibleTargetIds, checked);
      selectedRef.current = next;
      onSelectedIdsChange(next);
    },
    [disabledSet, onSelectedIdsChange],
  );

  const commitSingle = useCallback(
    (itemId: string, checked: boolean) => {
      anchorIdRef.current = itemId;
      commitSelection([itemId], checked);
    },
    [commitSelection],
  );

  const commitRange = useCallback(
    (itemId: string, checked: boolean) => {
      const anchorId = anchorIdRef.current ?? itemId;
      const anchorIndex = enabledItemIds.indexOf(anchorId);
      const itemIndex = enabledItemIds.indexOf(itemId);
      if (anchorIndex === -1 || itemIndex === -1) {
        commitSingle(itemId, checked);
        return;
      }

      const [start, end] =
        anchorIndex < itemIndex ? [anchorIndex, itemIndex] : [itemIndex, anchorIndex];
      commitSelection(enabledItemIds.slice(start, end + 1), checked);
    },
    [commitSelection, commitSingle, enabledItemIds],
  );

  const setAll = useCallback(
    (checked: boolean) => {
      commitSelection(enabledItemIds, checked);
    },
    [commitSelection, enabledItemIds],
  );

  const getSelectionCellProps = useCallback(
    (itemId: string) => ({
      onPointerDown: (event: PointerEvent<HTMLElement>) => {
        if (disabledSet.has(itemId) || (event.pointerType === "mouse" && event.button !== 0)) {
          return;
        }

        event.preventDefault();
        event.stopPropagation();
        suppressNextCheckedChangeRef.current =
          event.target instanceof HTMLElement && event.target.closest('[role="checkbox"]') !== null;
        const targetChecked = !selectedRef.current.includes(itemId);
        dragStateRef.current = { active: true, targetChecked };
        if (event.shiftKey) {
          commitRange(itemId, targetChecked);
        } else {
          commitSingle(itemId, targetChecked);
        }
      },
      onPointerEnter: (event: PointerEvent<HTMLElement>) => {
        if (!dragStateRef.current.active || disabledSet.has(itemId)) {
          return;
        }

        event.preventDefault();
        event.stopPropagation();
        commitSelection([itemId], dragStateRef.current.targetChecked);
      },
    }),
    [commitRange, commitSelection, commitSingle, disabledSet],
  );

  const getCheckboxProps = useCallback(
    (itemId: string) => ({
      checked: selectedSet.has(itemId),
      disabled: disabledSet.has(itemId),
      onCheckedChange: (checked: CheckedState) => {
        if (suppressNextCheckedChangeRef.current) {
          suppressNextCheckedChangeRef.current = false;
          return;
        }
        if (dragStateRef.current.active) {
          return;
        }
        commitSingle(itemId, checked === true);
      },
      onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => {
        if (!event.shiftKey || disabledSet.has(itemId)) {
          return;
        }
        if (event.key !== " " && event.key !== "Enter") {
          return;
        }

        event.preventDefault();
        event.stopPropagation();
        commitRange(itemId, !selectedRef.current.includes(itemId));
      },
    }),
    [commitRange, commitSingle, disabledSet, selectedSet],
  );

  const selectAllCheckboxProps = {
    checked: selectAllChecked,
    disabled: enabledItemIds.length === 0,
    onCheckedChange: (checked: CheckedState) => setAll(checked === true),
  };

  return {
    allSelected,
    someSelected,
    getSelectionCellProps,
    getCheckboxProps,
    selectAllCheckboxProps,
    setAll,
  };
}
