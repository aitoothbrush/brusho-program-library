import { PublicKey } from '@solana/web3.js';
import pkg from 'js-sha3';
import * as Collections from 'typescript-collections';
const { keccak_256 } = pkg;

const CACHE_EMPTY_NODE = new Map<number, Buffer>();
export const LEAF_BUFFER_LENGTH = 32;

export type MerkleTreeProof = {
    leafIndex: number;
    leaf: Buffer;
    proof: Buffer[];
    root: Buffer;
};

export class MerkleTree {
    leaves: TreeNode[];
    root: TreeNode;
    depth: number;

    /**
     * Please use `MerkleTree.sparseMerkleTreeFromLeaves` to
     * create trees instead. This method is exposed for testing purposes,
     * and for those that are familiar with the MerkleTree data structure.
     * @param leaves leaf nodes of the tree
     */
    constructor(leaves: Buffer[]) {
        const [nodes, finalLeaves] = buildLeaves(leaves);
        let seqNum = leaves.length;

        while (nodes.size() > 1) {
            const left = nodes.dequeue()!;
            const level = left.level;

            let right: TreeNode;
            if (level != nodes.peek()!.level) {
                right = emptyTreeNode(level, seqNum);
                seqNum++;
            } else {
                right = nodes.dequeue()!;
            }

            const parent: TreeNode = {
                id: seqNum,
                left: left,
                level: level + 1,
                node: hash(left.node, right.node),
                parent: undefined,
                right: right,
            };
            left.parent = parent;
            right.parent = parent;
            nodes.enqueue(parent);
            seqNum++;
        }

        this.leaves = finalLeaves;
        this.root = nodes.peek()!;
        this.depth = nodes.peek()!.level;
    }

    /**
     * This is the recommended way to create MerkleTrees.
     * If you're trying to match an on-chain MerkleTree,
     * set `depth` to `{@link ConcurrentMerkleTreeAccount}.getMaxDepth()`
     *
     * @param leaves leaves of the tree
     * @param depth number of levels in the tree
     * @returns MerkleTree
     */
    static sparseMerkleTreeFromLeaves(leaves: Buffer[], depth: number): MerkleTree {
        const _leaves: Buffer[] = [];
        for (let i = 0; i < 2 ** depth; i++) {
            if (i < leaves.length) {
                _leaves.push(leaves[i]);
            } else {
                _leaves.push(Buffer.alloc(32));
            }
        }
        return new MerkleTree(_leaves);
    }

    getRoot(): Buffer {
        return this.root.node;
    }

    getProof(leafIndex: number, minimizeProofHeight = false, treeHeight = -1, verbose = false): MerkleTreeProof {
        const proof: TreeNode[] = [];

        let node = this.leaves[leafIndex];

        let height = 0;
        while (typeof node.parent !== 'undefined') {
            if (minimizeProofHeight && height >= treeHeight) {
                break;
            }
            if (verbose) {
                console.log(`${node.level}: ${Uint8Array.from(node.node)}`);
            }
            const parent = node.parent;
            if (parent.left!.id === node.id) {
                proof.push(parent.right!);

                const hashed = hash(node.node, parent.right!.node);
                if (!hashed.equals(parent.node)) {
                    console.log(hashed);
                    console.log(parent.node);
                    throw new Error('Invariant broken when hashing left node');
                }
            } else {
                proof.push(parent.left!);

                const hashed = hash(parent.left!.node, node.node);
                if (!hashed.equals(parent.node)) {
                    console.log(hashed);
                    console.log(parent.node);
                    throw new Error('Invariant broken when hashing right node');
                }
            }
            node = parent;
            height++;
        }

        return {
            leaf: this.leaves[leafIndex].node,
            leafIndex,
            proof: proof.map(treeNode => treeNode.node),
            root: this.getRoot(),
        };
    }

    updateLeaf(leafIndex: number, newLeaf: Buffer, verbose = false) {
        const leaf = this.leaves[leafIndex];
        leaf.node = newLeaf;
        let node = leaf;

        let i = 0;
        while (typeof node.parent !== 'undefined') {
            if (verbose) {
                console.log(`${i}: ${Uint8Array.from(node.node)}`);
            }
            node = node.parent;
            node.node = hash(node.left!.node, node.right!.node);
            i++;
        }
        if (verbose) {
            console.log(`${i}: ${Uint8Array.from(node.node)}`);
        }
        this.root = node;
    }

    static hashProof(merkleTreeProof: MerkleTreeProof, verbose = false): Buffer {
        const { leaf, leafIndex, proof } = merkleTreeProof;

        let node = new PublicKey(leaf).toBuffer();
        for (let i = 0; i < proof.length; i++) {
            if ((leafIndex >> i) % 2 === 0) {
                node = hash(node, new PublicKey(proof[i]).toBuffer());
            } else {
                node = hash(new PublicKey(proof[i]).toBuffer(), node);
            }
            if (verbose) console.log(`node ${i} ${new PublicKey(node).toString()}`);
        }
        return node;
    }

    flatten(): Buffer[] {
        const nodes: Buffer[] = [];

        let levelTreeNodes: TreeNode[] = [this.root];
        while (levelTreeNodes.length > 0) {
            const nextLevelTreeNodes: TreeNode[] = [];
            for (let treeNode of levelTreeNodes) {
                nodes.push(treeNode.node);

                if (treeNode.left) {
                    nextLevelTreeNodes.push(treeNode.left);
                }
                if (treeNode.right) {
                    nextLevelTreeNodes.push(treeNode.right);
                }
            }

            levelTreeNodes = nextLevelTreeNodes;
        }

        return nodes;
    }

    getCanopy(canopyDepth: number): Buffer[] {
        if (canopyDepth < 1 || canopyDepth > this.root.level) {
            throw new Error(`Illegal canopy depth ${canopyDepth}, tree's depth is ${this.root.level}`)
        }

        const lowestLevel = this.root.level - canopyDepth;
        const canopy: Buffer[] = [];

        let levelTreeNodes: TreeNode[] = [this.root.left!, this.root.right!];
        while (levelTreeNodes.length > 0) {
            canopy.push(...levelTreeNodes.map(treeNode => treeNode.node));

            if (levelTreeNodes[0].level == lowestLevel) {
                break;
            }

            const nextLevelTreeNodes: TreeNode[] = [];
            for (let treeNode of levelTreeNodes) {
                if (treeNode.left) {
                    nextLevelTreeNodes.push(treeNode.left);
                }

                if (treeNode.right) {
                    nextLevelTreeNodes.push(treeNode.right);
                }
            }

            levelTreeNodes = nextLevelTreeNodes;
        }

        return canopy;
    }

    /**
     * Verifies that a root matches the proof.
     * @param root Root of a MerkleTree
     * @param merkleTreeProof Proof to a leaf in the MerkleTree
     * @param verbose Whether to print hashed nodes
     * @returns Whether the proof is valid
     */
    static verify(root: Buffer, merkleTreeProof: MerkleTreeProof, verbose = false): boolean {
        const node = MerkleTree.hashProof(merkleTreeProof, verbose);
        const rehashed = new PublicKey(node).toString();
        const received = new PublicKey(root).toString();
        if (rehashed !== received) {
            if (verbose) console.log(`Roots don't match! Expected ${rehashed} got ${received}`);
            return false;
        }
        if (verbose) console.log(`Hashed ${rehashed} got ${received}`);
        return rehashed === received;
    }
}

export type TreeNode = {
    node: Buffer;
    left: TreeNode | undefined;
    right: TreeNode | undefined;
    parent: TreeNode | undefined;
    level: number;
    id: number;
};

/**
 * Uses on-chain hash fn to hash together buffers
 */
export function hash(...buffers: Buffer[]): Buffer {
    return Buffer.from(keccak_256.digest(Buffer.concat(buffers)));
}

/**
 * Creates the leaf node in a tree of empty leaves of height `level`.
 * Uses {@link CACHE_EMPTY_NODE} to efficiently produce
 * @param level
 * @returns
 */
export function emptyNode(level: number): Buffer {
    if (CACHE_EMPTY_NODE.has(level)) {
        return CACHE_EMPTY_NODE.get(level)!;
    }
    if (level == 0) {
        return Buffer.alloc(32);
    }

    const result = hash(emptyNode(level - 1), emptyNode(level - 1));
    CACHE_EMPTY_NODE.set(level, result);
    return result;
}

/**
 * Helper function when creating a MerkleTree
 * @param level
 * @param id
 * @returns
 */
function emptyTreeNode(level: number, id: number): TreeNode {
    return {
        id,
        left: undefined,
        level: level,
        node: emptyNode(level),
        parent: undefined,
        right: undefined,
    };
}

/**
 * Helper function to build a MerkleTree
 * @param leaves
 * @returns
 */
function buildLeaves(leaves: Buffer[]): [Collections.Queue<TreeNode>, TreeNode[]] {
    const nodes = new Collections.Queue<TreeNode>();
    const finalLeaves: TreeNode[] = [];
    leaves.forEach((buffer, index) => {
        if (buffer.length != LEAF_BUFFER_LENGTH) {
            throw Error(
                `Provided leaf has length: ${buffer.length}, but we need all leaves to be length ${LEAF_BUFFER_LENGTH}`,
            );
        }

        const treeNode = {
            id: index,
            left: undefined,
            level: 0,
            node: buffer,
            parent: undefined,
            right: undefined,
        };
        nodes.enqueue(treeNode);
        finalLeaves.push(treeNode);
    });
    return [nodes, finalLeaves];
}
