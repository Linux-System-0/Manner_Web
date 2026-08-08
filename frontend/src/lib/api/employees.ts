import { client } from './client'
import type {
  ApiResponse,
  PaginatedData,
  PaginatedResponse,
  Employee,
  EmployeeQueryParams,
  CreateEmployeeRequest,
  UpdateEmployeeRequest,
} from '@/types'

export async function getEmployees(
  params: EmployeeQueryParams = {},
): Promise<PaginatedResponse<Employee>> {
  const res = await client.get<PaginatedData<Employee>>('/employees', params as Record<string, never>)
  return res
}

export async function getEmployee(
  id: string,
): Promise<ApiResponse<Employee>> {
  const res = await client.get<Employee>(`/employees/${id}`)
  return res
}

export async function createEmployee(
  data: CreateEmployeeRequest,
): Promise<ApiResponse<Employee>> {
  const res = await client.post<Employee>('/employees', data)
  return res
}

export async function updateEmployee(
  id: string,
  data: UpdateEmployeeRequest,
): Promise<ApiResponse<Employee>> {
  const res = await client.put<Employee>(`/employees/${id}`, data)
  return res
}

export async function deleteEmployee(
  id: string,
): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/employees/${id}`)
  return res
}

export async function resetPassword(
  id: string,
  new_password: string,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/employees/${id}/password`, {
    new_password,
  })
  return res
}

export async function updateEmployeePermissions(
  id: string,
  permission_codes: string[],
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/employees/${id}/permissions`, {
    permission_codes,
  })
  return res
}
