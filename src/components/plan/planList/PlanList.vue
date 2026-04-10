<script setup lang="ts">
import type { Plan } from '@/types/plan'
import PlanCreateDialog from '../PlanCreateDialog.vue'
import PlanEditDialog from '../PlanEditDialog.vue'
import PlanListEmptyState from '../PlanListEmptyState.vue'
import PlanListHeader from '../PlanListHeader.vue'
import PlanListItem from '../PlanListItem.vue'
import PlanListStatusTabs from '../PlanListStatusTabs.vue'
import PlanSplitConfigDialog from '../PlanSplitConfigDialog.vue'
import TaskSplitDialog from '../taskSplitDialog/TaskSplitDialog.vue'
import { usePlanListView } from './usePlanList'

const emit = defineEmits<{
  (e: 'plan-click', plan: Plan): void
}>()

const {
  activeStatusTab,
  activeStatusTabLabel,
  agentOptions,
  canSaveDraft,
  canStartSplitFromCreate,
  canStartSplitFromList,
  closeCreateDialog,
  closeEditDialog,
  closeSplitConfigDialog,
  confirmSplitConfigAndStart,
  createDialogModelOptions,
  createForm,
  createManualPlan,
  createPlan,
  deletePlan,
  editDialogModelOptions,
  editForm,
  editingPlan,
  handleListProjectChange,
  openCreateDialog,
  openEditDialog,
  planItems,
  plans,
  projectOptions,
  saveEdit,
  selectPlan,
  selectedListProject,
  selectedProjectIdForList,
  showCreateDialog,
  showEditDialog,
  showSplitConfigDialog,
  splitConfigForm,
  splitConfigModelOptions,
  splitConfigPlan,
  startSplitTasks,
  statusTabCounts,
  statusTabs,
  updateCreateForm,
  updateEditForm,
  updateSplitConfigForm,
  visiblePlanCount
} = usePlanListView(emit)
</script>

<template>
  <div class="plan-list">
    <PlanListHeader
      :visible-plan-count="visiblePlanCount"
      :selected-project-id="selectedProjectIdForList"
      :selected-project-path="selectedListProject?.path || ''"
      :project-options="projectOptions"
      @create="openCreateDialog"
      @update:selected-project-id="selectedProjectIdForList = $event; handleListProjectChange($event)"
    />

    <div class="list-body">
      <PlanListStatusTabs
        :tabs="statusTabs"
        :active-tab="activeStatusTab"
        :counts="statusTabCounts"
        @update:active-tab="activeStatusTab = $event"
      />

      <div
        v-if="planItems.length > 0"
        class="plan-items"
      >
        <PlanListItem
          v-for="item in planItems"
          :key="item.plan.id"
          :item="item"
          @select="selectPlan(item.plan)"
          @split="startSplitTasks(item.plan)"
          @edit="openEditDialog(item.plan)"
          @delete="deletePlan(item.plan)"
        />
      </div>

      <PlanListEmptyState
        v-else
        :title="plans.length === 0 ? '暂无计划' : `${activeStatusTabLabel}暂无计划`"
      />
    </div>

    <PlanCreateDialog
      :visible="showCreateDialog"
      :form="createForm"
      :agent-options="agentOptions"
      :model-options="createDialogModelOptions"
      :can-save-draft="canSaveDraft"
      :can-start-split="canStartSplitFromCreate"
      @update:form="updateCreateForm"
      @close="closeCreateDialog"
      @save-draft="createPlan(false)"
      @start-split="createPlan(true)"
      @create-manual="createManualPlan"
    />

    <PlanEditDialog
      :visible="showEditDialog"
      :plan="editingPlan"
      :form="editForm"
      :agent-options="agentOptions"
      :model-options="editDialogModelOptions"
      @update:form="updateEditForm"
      @close="closeEditDialog"
      @save="saveEdit"
    />

    <PlanSplitConfigDialog
      :visible="showSplitConfigDialog"
      :plan="splitConfigPlan"
      :form="splitConfigForm"
      :agent-options="agentOptions"
      :model-options="splitConfigModelOptions"
      :can-start="canStartSplitFromList"
      @update:form="updateSplitConfigForm"
      @close="closeSplitConfigDialog"
      @start="confirmSplitConfigAndStart"
    />

    <TaskSplitDialog />
  </div>
</template>

<style scoped src="./styles.css"></style>
