#include <float.h>

#define SHARED_ELEMS 1024

extern "C" __global__ void bitonic_step(
    float* __restrict__ data,
    int n_pad,
    int j,
    int k
) {
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned int ixj = i ^ (unsigned int)j;
    if (ixj > i && (int)i < n_pad) {
        float a = data[i];
        float b = data[ixj];
        bool asc = ((i & (unsigned int)k) == 0u);
        if ((asc && a > b) || (!asc && a < b)) {
            data[i] = b;
            data[ixj] = a;
        }
    }
}

extern "C" __global__ void bitonic_shared_stage(
    float* __restrict__ data,
    int n_pad,
    int k
) {
    __shared__ float tile[SHARED_ELEMS];
    int tid = (int)threadIdx.x;
    int base = (int)blockIdx.x * k;
    if (base >= n_pad || tid >= k) {
        return;
    }

    int gi = base + tid;
    tile[tid] = (gi < n_pad) ? data[gi] : FLT_MAX;
    __syncthreads();

    for (int j = k >> 1; j > 0; j >>= 1) {
        int ixj = tid ^ j;
        if (ixj > tid && ixj < k) {
            float a = tile[tid];
            float b = tile[ixj];
            bool asc = (((unsigned int)(base + tid) & (unsigned int)k) == 0u);
            if ((asc && a > b) || (!asc && a < b)) {
                tile[tid] = b;
                tile[ixj] = a;
            }
        }
        __syncthreads();
    }

    if (gi < n_pad) {
        data[gi] = tile[tid];
    }
}

extern "C" __global__ void bitonic_shared_finish(
    float* __restrict__ data,
    int n_pad,
    int k,
    int j_start
) {
    __shared__ float tile[SHARED_ELEMS];
    int tid = (int)threadIdx.x;
    int base = (int)blockIdx.x * SHARED_ELEMS;
    if (base >= n_pad) {
        return;
    }

    int gi = base + tid;
    tile[tid] = (gi < n_pad) ? data[gi] : FLT_MAX;
    __syncthreads();

    for (int j = j_start; j > 0; j >>= 1) {
        int ixj = tid ^ j;
        if (ixj > tid) {
            float a = tile[tid];
            float b = tile[ixj];
            bool asc = (((unsigned int)gi & (unsigned int)k) == 0u);
            if ((asc && a > b) || (!asc && a < b)) {
                tile[tid] = b;
                tile[ixj] = a;
            }
        }
        __syncthreads();
    }

    if (gi < n_pad) {
        data[gi] = tile[tid];
    }
}

extern "C" __global__ void extract_percentiles(
    const float* __restrict__ sorted,
    float* __restrict__ out3,
    int n
) {
    if (threadIdx.x == 0 && blockIdx.x == 0) {
        int i25 = (int)(0.25 * (double)n);
        int i50 = (int)(0.50 * (double)n);
        int i75 = (int)(0.75 * (double)n);
        if (i25 >= n) i25 = n - 1;
        if (i50 >= n) i50 = n - 1;
        if (i75 >= n) i75 = n - 1;
        out3[0] = sorted[i25];
        out3[1] = sorted[i50];
        out3[2] = sorted[i75];
    }
}
